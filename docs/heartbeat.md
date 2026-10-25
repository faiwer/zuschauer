# Heartbeat

Zuschauer can't tell you that zuschauer itself stopped, or that the whole
machine went down. A heartbeat covers that: zuschauer sends a request to an
outside service every minute, and that service alerts you when the requests
stop coming.

- [How it works](#how-it-works)
- [Setting it up](#setting-it-up)
- [Keys](#keys)
- [Templates](#templates)
- [Examples](#examples)
- [Failures](#failures)

## How it works

Services like [healthchecks.io](https://healthchecks.io),
[Uptime Kuma](https://github.com/louislam/uptime-kuma)'s push monitors, or
[Better Stack](https://betterstack.com/uptime)'s heartbeats give you a URL
and expect it to be requested every so often. When a request doesn't arrive
in time, they notify you.

That's the case zuschauer can't report on its own: a crashed process, a
kernel panic, a power cut, or a lost network connection. In each one, the
heartbeat simply stops.

## Setting it up

1. Create a check in the outside service, with a period a bit longer than
   the heartbeat's `every`. With the default of one minute, a period of
   2 minutes, plus whatever grace time the service offers, avoids alerts for
   one slow request.
2. Copy the URL it gives you into `heartbeat.url`:

   ```jsonc
   "heartbeat": {
     "url": "https://hc-ping.com/<your-check-uuid>"
   }
   ```

3. Run `zuschauer --check` to validate the config, then restart zuschauer.
   The first request goes out right away, so the service shows the check as
   up within seconds.

The heartbeat has one indicator in the dashboard's header: green once a
request got through, red while they fail. Hover it for the details.

## Keys

| Key | Default | |
| --- | --- | --- |
| `url` | required | An `http://` or `https://` URL. |
| `method` | `"GET"` | `"GET"`, `"HEAD"`, `"POST"`, `"PUT"` or `"PATCH"`. |
| `headers` | none | Extra headers, each as `"Name: value"`. |
| `body` | none | The request body. Only with `POST`, `PUT` or `PATCH`. |
| `every` | `"1m"` | How often the request is sent. |

A few headers are set by zuschauer and can't be changed: `Host`,
`Content-Length`, `Connection` and `Transfer-Encoding`. Your own `User-Agent`
and `Accept` replace the defaults. `Content-Type` isn't set, so add it when
the service needs one.

Without a `heartbeat` section, nothing is sent.

## Templates

`url`, `headers` and `body` can include the [`context`](configuration.md#top-level-settings),
the name of this server in notifications. That's useful when one check in the
outside service is shared by several servers, or to say which one is pinging:

| Placeholder | Becomes |
| --- | --- |
| `%{context}` | the context as it is |
| `%{context\|url}` | the context percent-encoded, for inside a URL |
| `%{context\|json}` | the context escaped for inside a JSON string |

For example, with `"context": "My server"`:

```jsonc
"url": "https://kuma.example.com/api/push/<token>?msg=%{context|url}"
```

sends `?msg=My%20server`. Anything else in the text, like `%20`, is left as
it is.

## Examples

### healthchecks.io

```jsonc
"heartbeat": {
  "url": "https://hc-ping.com/<your-check-uuid>"
}
```

### Uptime Kuma

A push monitor, with the server's name as the message:

```jsonc
"heartbeat": {
  "url": "https://kuma.example.com/api/push/<token>?status=up&msg=%{context|url}",
  "every": "30s"
}
```

### Better Stack

```jsonc
"heartbeat": {
  "url": "https://uptime.betterstack.com/api/v1/heartbeat/<token>"
}
```

### A JSON webhook of your own

```jsonc
"heartbeat": {
  "url": "https://hooks.example.com/zuschauer",
  "method": "POST",
  "headers": [
    "Authorization: Bearer <token>",
    "Content-Type: application/json"
  ],
  "body": "{ \"server\": \"%{context|json}\" }"
}
```

## Failures

A request succeeds when it gets a `2xx` answer within 10 seconds, or within
`every` if that's shorter. Anything else fails it: no connection, no answer
in time, a TLS error, or another status code. Redirects aren't followed, so a
`301` counts as a failure; use the URL it points to.

A failed request isn't retried. The next one goes out at the next `every`, as
usual. Zuschauer doesn't notify you about its own heartbeat failing: if the
outside service doesn't hear from it for long enough, the service does.

Failures are logged when they start and when requests get through again, not
once per request:

```text
heartbeat: failing, no response within 10s
heartbeat: sent
```
