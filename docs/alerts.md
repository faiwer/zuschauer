# Alerts

An alert turns something zuschauer watches into a notification: it fires when
a problem lasts long enough, and tells you again when it's resolved. Every
watched thing can have one, from CPU usage to a nightly backup job.

- [Two kinds](#two-kinds)
- [How an alert fires](#how-an-alert-fires)
- [What you receive](#what-you-receive)
- [`for`](#for)
- [`resolveBelow`](#resolvebelow)
- [`severity`](#severity)
- [IDs and `dependsOn`](#ids-and-dependson)
- [Where alerts go](#where-alerts-go)
- [Restarts](#restarts)

## Two kinds

**Threshold alerts** watch a number: CPU, memory, storage and temperature,
see [host](host.md#threshold-alerts).

```jsonc
"cpu": {
  "alert": { "above": "90%", "for": "5m", "resolveBelow": "70%" }
}
```

**State alerts** watch whether something works: containers, health checks,
certificates, services, commands and jobs, see [apps](apps.md#alerts).

```jsonc
"containers": {
  "blog-db": { "alert": { "for": "1m" } }
}
```

Both take the same keys, except for the numbers:

| Key | Threshold alerts | State alerts | |
| --- | --- | --- | --- |
| `above` | required | — | The value that's a problem when exceeded. |
| `resolveBelow` | `above` | — | The value it must drop below to resolve. |
| `for` | required | `"0s"` | How long the problem must last before it fires. |
| `severity` | `"warning"` | `"warning"` | `"critical"`, `"warning"` or `"info"`. |
| `id` | generated | generated | A name for `dependsOn` to refer to. |
| `dependsOn` | none | none | IDs of alerts that hold this one back. |

Services also take `restarts`, see [restart loops](apps.md#restart-loops).

Anything without an `alert` is still watched and shown on the dashboard; it
just never notifies.

## How an alert fires

An alert is in one of three states:

- **OK:** no problem.
- **Pending:** there's a problem, but it hasn't lasted `for` yet.
- **Firing:** the problem lasted `for`. A notification went out.

Only two moments are announced: when an alert starts firing, and when a firing
alert is resolved. A problem that clears while it's still pending is never
announced, which is the point of `for`.

For example, with `"above": "90%", "for": "5m"` and CPU read every minute:

| Time | CPU | State | Sent |
| --- | --- | --- | --- |
| 10:00 | 40% | OK | |
| 10:01 | 95% | pending | |
| 10:03 | 60% | OK | |
| 10:10 | 95% | pending | |
| 10:15 | 97% | firing | CPU: 97.0%, above 90%, for 5m |
| 10:22 | 50% | OK | CPU resolved: after 12m |

The spike at 10:01 clears before 5 minutes pass, so nothing is sent. The one
at 10:10 lasts, so it fires at 10:15. Every reading in between must be a
problem: one good reading resets the clock.

Nothing more is sent while an alert keeps firing. There are no reminders, and
a change of reason, like a stopped service starting to fail, doesn't notify
again.

## What you receive

A firing alert's notification has:

- a title made of the [context](configuration.md#top-level-settings) and
  what's watched: `[my-server] Blog container blog-db`;
- the reason, and how long the problem has lasted:
  `not running, for 1m`.

A resolved alert's is quieter:

- `[my-server] Blog container blog-db resolved`;
- `after 14m`, counted from when the problem started.

Some example reasons:

| Watched | Reason |
| --- | --- |
| CPU | `97.0%, above 90%` |
| Storage | `91.3%, above 85%` |
| Temperature | `82.0°C, above 75°C` |
| Container | `not running`, `paused` |
| Health check | `got 502, expected 200`, `content type text/html, expected application/json` |
| Certificate | `expires in 9d` |
| Service | `failed, exit code 1`, `restarted 5 times in 1h` |
| Job | `didn't start, due at 2026-10-26 03:00`, `failed at 2026-10-26 03:42 after 42m` |

## `for`

How long a problem must last before the alert fires. It's the main tool
against noise: most problems that clear within a minute or two aren't worth
waking up for.

- **Threshold alerts** require it, and it can't be shorter than the reading's
  `every`, or a single reading would fire it.
- **State alerts** default to `"0s"`: the first failed check fires. Set it for
  things that fail briefly on their own, like a container restarted by an
  update, or a health check that times out once in a while.

Some problems fire at once, whatever `for` says, because waiting wouldn't
make them better:

- a service [restarting too often](apps.md#restart-loops);
- a [job](apps.md#what-counts-as-a-problem) that hung or failed.

A late `job` does wait for `for`, which makes it a grace period for late
starts.

## `resolveBelow`

By default, a threshold alert resolves as soon as the value is back at or
under `above`. A value that hovers around the threshold then fires and
resolves again and again.

`resolveBelow` sets a lower bar for resolving:

```jsonc
"memory": {
  "alert": { "above": "90%", "for": "5m", "resolveBelow": "80%" }
}
```

This fires above 90%, and resolves only once memory drops under 80%. Between
the two, a firing alert keeps firing. It can't be higher than `above`.

## `severity`

How much the alert matters:

| Severity | For |
| --- | --- |
| `"critical"` | Needs you now: the site is down, the disk is full. |
| `"warning"` | The default. Needs a look soon. |
| `"info"` | Worth knowing about, nothing to fix right away. |

Notifiers use it to decide how loud a notification is, see
[notifiers](notifiers.md). Resolved alerts are always sent quietly.

## IDs and `dependsOn`

One failure often causes others. When the database container stops, the
site's health check fails too, and two notifications arrive for one problem.
`dependsOn` lets an alert wait while another one has a problem:

```jsonc
"apps": {
  "blog": {
    "containers": {
      "blog-db": { "alert": { "for": "1m", "id": "blog-db", "severity": "critical" } }
    },
    "healthChecks": [
      {
        "name": "site",
        "url": "https://example.com/",
        "alert": { "for": "2m", "dependsOn": ["blog-db"] }
      }
    ]
  }
}
```

Every alert has an ID. Give the ones others depend on an `id` of your own,
like `blog-db` above, rather than relying on the generated one. IDs must be
unique across the whole config.

How it works:

- While `blog-db` has a problem, pending or firing, the health check's alert
  stays pending, however long its own problem lasts.
- When `blog-db` is resolved and the site is fine again, only the database's
  two notifications were sent.
- If the site is still failing once `blog-db` is resolved, its alert fires
  right away, without waiting for its next check, as long as its own problem
  lasted `for`.
- An alert that was already firing before the one it depends on got a
  problem keeps firing.

Dependencies chain: if the database depends on a `storage` alert, a full disk
holds back both the database's alert and the site's. An alert can depend on
several others, and waits while any of them has a problem.

The config check rejects an ID used twice, a `dependsOn` naming an ID that
doesn't exist, and alerts that depend on each other in a loop:

```text
alert ID "blog/site" is used twice, give one an "id"
alert "blog/site": dependsOn "blog-database" is not an alert ID
alert "blog-db" depends on itself
```

## Where alerts go

Each notification goes to every configured [notifier](notifiers.md). If one
can't be reached, its notifications are queued and retried, in order, so a
resolve never arrives before its alert.

Firing alerts are also listed in the dashboard's Alerts panel, with their
severity and how long they've lasted. Without notifiers, that panel and the
log are the only places they show up, and zuschauer says so when it starts:

```text
alerts: no notifiers configured, alerts only show on the page
```

Every change is logged:

```text
alert blog-db: firing, not running, for 1m
alert blog-db: resolved
```

## Restarts

Alerts are kept in memory only. After zuschauer restarts, every alert starts
as OK:

- a problem that's still there fires again once it lasts `for`;
- a problem that cleared while zuschauer was down is never announced as
  resolved.
