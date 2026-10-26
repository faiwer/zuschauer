# Notifiers

Notifiers deliver [alerts](alerts.md) to you. Zuschauer supports two kinds:

- [ntfy](https://ntfy.sh): a simple publish-subscribe notification service with
  apps for Android, iOS and the desktop, which you can use for free at ntfy.sh
  or host yourself.
- [Webhooks](#webhooks): an HTTP request to a URL of your choosing, for chat
  services, your own scripts, or anything else that takes one.

- [Setting up ntfy](#setting-up-ntfy)
- [Keys](#keys)
- [Choosing a topic](#choosing-a-topic)
- [A self-hosted server](#a-self-hosted-server)
- [How notifications look](#how-notifications-look)
- [Webhooks](#webhooks)
- [Several notifiers](#several-notifiers)
- [Delivery](#delivery)

## Setting up ntfy

1. Install the ntfy app and subscribe to a topic: a name you make up, like
   `my-server-alerts-7f3k2q`.
2. Add a notifier with the same topic:

   ```jsonc
   "notifiers": {
     "phone": {
       "type": "ntfy",
       "topic": "my-server-alerts-7f3k2q"
     }
   }
   ```

3. Check that the topic reaches you, by publishing to it yourself:

   ```sh
   curl -d "Hello from my-server" https://ntfy.sh/my-server-alerts-7f3k2q
   ```

4. Run `zuschauer --check` to validate the config, then restart zuschauer.

`notifiers` is keyed by a name of your choosing, here `phone`. It only appears
in zuschauer's log.

## Keys

| Key | Default | |
| --- | --- | --- |
| `type` | required | `"ntfy"`. |
| `topic` | required | Up to 64 letters, digits, `-` and `_`. |
| `address` | `"https://ntfy.sh"` | The ntfy server, see [self-hosted](#a-self-hosted-server). |
| `token` | none | An access token, for servers that require one. |

## Choosing a topic

On the public ntfy.sh, a topic isn't protected by anything but its name:
anyone who knows it can read your notifications or send fake ones. So pick a
name nobody would guess, the way you'd pick a password:

```sh
echo "my-server-$(openssl rand -hex 6)"
```

Notifications say what's down and where, so treat the topic like a secret: keep
it out of public repositories and screenshots. If that's not enough, reserve
the topic with an ntfy.sh account, or host your own server.

## A self-hosted server

Point `address` at your server, and give an access token if it requires one:

```jsonc
"notifiers": {
  "phone": {
    "type": "ntfy",
    "address": "https://ntfy.example.com",
    "topic": "alerts",
    "token": "tk_<your-access-token>"
  }
}
```

- `address` is the server's URL, without a query string.
- `token` is sent as `Authorization: Bearer <token>`. Create one with
  `ntfy token add <user>` on the server, or in the web app's account
  settings. Username and password login isn't supported; use a token.
- With a token in it, the config file holds a secret: keep it readable only by
  zuschauer, see [installation](installation.md).

## How notifications look

Each notification's title starts with the
[context](configuration.md#top-level-settings), so you can tell machines
apart:

```text
🚨 [my-server] Blog container blog-db
not running, for 1m
```

```text
✅ [my-server] Blog container blog-db resolved
after 14m
```

The alert's [severity](alerts.md#severity) sets ntfy's priority and the icon:

| Severity | ntfy priority | Icon |
| --- | --- | --- |
| `"critical"` | 5, max: a long vibration and a pop-over on most phones | 🚨 |
| `"warning"` | 4, high | ⚠️ |
| `"info"` | 3, default | ℹ️ |
| resolved | 2, low: no sound or vibration | ✅ |

The ntfy app lets you change what each priority does, for example to let
critical notifications through Do Not Disturb.

## Webhooks

A webhook sends each alert change, firing or resolved, as an HTTP request:

```jsonc
"notifiers": {
  "hook": {
    "type": "webhook",
    "url": "https://hooks.example.com/zuschauer",
    "headers": ["Authorization: Bearer <token>"],
    "hmac": { "secret": "<a long random string>" }
  }
}
```

| Key | Default | |
| --- | --- | --- |
| `type` | required | `"webhook"`. |
| `url` | required | Where to send it, `http://` or `https://`. Can include `%{context}`, as in the [heartbeat](heartbeat.md#templates). |
| `method` | `"POST"` | `"POST"`, `"PUT"` or `"PATCH"`. |
| `headers` | none | `"Name: value"` strings. Can include `%{context}`. |
| `body` | the JSON below | A [template](#a-body-of-your-own) for the body. |
| `hmac.secret` | none | Signs the body, see [signatures](#signatures). |
| `hmac.header` | `"X-Signature"` | The header the signature goes in. |

`Content-Type` is `application/json` unless `headers` sets it. `Host`,
`Content-Length`, `Connection` and `Transfer-Encoding` are zuschauer's own and
can't be set.

Without `body`, zuschauer sends:

```json
{
  "context": "my-server",
  "alert": "blog-db",
  "status": "firing",
  "severity": "critical",
  "title": "Blog container blog-db",
  "message": "not running, for 1m"
}
```

- `alert` is the alert's [ID](alerts.md#ids-and-dependson), the same in the
  firing and the resolved request.
- `status` is `"firing"` or `"resolved"`.
- `severity` is `"critical"`, `"warning"` or `"info"`, and `null` when
  resolved.
- `title` and `message` are what ntfy would show, without the context.

### A body of your own

Most chat services want their own JSON. `body` is a template with these
placeholders, each also as `%{name|json}`, escaped for inside a JSON string,
and `%{name|url}`, percent-encoded:

| Placeholder | Becomes |
| --- | --- |
| `%{context}` | the [context](configuration.md#top-level-settings) |
| `%{alert}` | the alert's ID |
| `%{status}` | `firing` or `resolved` |
| `%{severity}` | `critical`, `warning` or `info`; empty when resolved |
| `%{title}` | the notification's title |
| `%{message}` | the notification's text |

For example, a Slack or Mattermost incoming webhook:

```jsonc
"notifiers": {
  "chat": {
    "type": "webhook",
    "url": "https://hooks.slack.com/services/<your-webhook-path>",
    "body": "{ \"text\": \"[%{context|json}] %{title|json}: %{message|json}\" }"
  }
}
```

Use `|json` for anything inside a JSON string: a title with a `"` in it would
break the JSON otherwise.

### Signatures

With `hmac`, zuschauer signs every body with HMAC-SHA256 and the secret, and
sends the signature in a header:

```text
X-Signature: sha256=5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843
```

The receiver computes the same from the body it got and its copy of the
secret, and drops the request if they differ. That proves the request came
from someone who knows the secret and that the body wasn't changed on the way.
It's the same format GitHub uses, so code that checks GitHub's
`X-Hub-Signature-256` works with `"header": "X-Hub-Signature-256"`. In Python:

```python
import hmac, hashlib

def is_genuine(body: bytes, header: str, secret: bytes) -> bool:
    expected = "sha256=" + hmac.new(secret, body, hashlib.sha256).hexdigest()
    return hmac.compare_digest(expected, header)
```

Sign the body exactly as received, before parsing it. Like a token, the secret
makes the config file hold a secret: keep it readable only by zuschauer, see
[installation](installation.md).

## Several notifiers

Every notification goes to every notifier. To get alerts on two topics, say
yours and a teammate's, or on ntfy and a webhook, add one notifier for each:

```jsonc
"notifiers": {
  "me": { "type": "ntfy", "topic": "my-server-alerts-7f3k2q" },
  "team": {
    "type": "ntfy",
    "address": "https://ntfy.example.com",
    "topic": "ops",
    "token": "tk_<your-access-token>"
  }
}
```

There's no routing by severity or app yet: each notifier gets them all.

Without any notifiers, alerts still show on the dashboard and in the log,
and zuschauer says so when it starts.

## Delivery

Each notifier has its own queue, so a slow or unreachable one doesn't hold up
the others. Within a notifier, notifications go out one at a time, in order,
so a "resolved" never arrives before the alert it resolves.

When a notification can't be delivered:

- **The server can't be reached, doesn't answer within 15 seconds, is
  overloaded (`429`) or fails (`5xx`):** it's retried after 5 seconds, then
  after twice as long each time, up to every 5 minutes, until it gets through.
- **The server refuses it with another status, like `401` for a wrong
  token:** it's dropped, since sending it again won't help.
- **More than 64 notifications are waiting:** new ones are dropped until the
  queue drains.

Each of these is logged, with the server's answer:

```text
notifier phone: sent "Blog container blog-db"
notifier phone: no response within 15s, retrying in 10s
notifier phone: dropped "Blog container blog-db", HTTP 401: {"code":40101,"http":401,"error":"unauthorized", …}
```

A wrong token or topic only shows up on the first delivery, so after setting
up a notifier, check the log the first time an alert fires.
