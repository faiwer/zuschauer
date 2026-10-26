# Configuration

Zuschauer reads one file, `config.jsonc` in the current directory unless you
pass another path:

```sh
zuschauer /etc/zuschauer/config.jsonc
```

The config is read once at startup. To apply changes, restart zuschauer.

- [The format](#the-format)
- [Values](#values)
- [Top-level settings](#top-level-settings)
- [Sections](#sections)
- [Defaults](#defaults)
- [Checking the config](#checking-the-config)
- [A complete example](#a-complete-example)

## The format

The file is JSON with comments. All three comment styles work:

```jsonc
{
  // A line comment.
  /* A block comment. */
  # A hash comment.
  "listen": "127.0.0.1:9101"
}
```

Beyond comments it's strict JSON:

- **No trailing commas.** `{ "a": 1, }` is an error.
- **Keys are camelCase**: `healthChecks`, `warnBefore`, `dependsOn`.
- **Unknown keys are errors**, so a typo doesn't silently turn a setting off:

  ```text
  config.jsonc: hots: unknown field `hots`, expected one of `listen`, `context`, `host`, `docker`, `certificates`, `services`, `apps`, `notifiers`, `heartbeat` at line 3 column 9
  ```

Everything except `listen` is optional. A section you leave out is either off
or uses its defaults, see [Defaults](#defaults).

## Values

Durations, percentages and temperatures are strings with a unit, so a number
is never ambiguous.

### Durations

A number and a unit, like `"30s"`, `"10m"` or `"14d"`. Several parts can be
combined: `"1h 30m"`.

| Unit | Means |
| --- | --- |
| `ms` | milliseconds |
| `s`, `sec` | seconds |
| `m`, `min` | minutes |
| `h`, `hr` | hours |
| `d`, `days` | days |
| `w`, `weeks` | weeks |

Long forms work too: `"5 minutes"`, `"2 hours"`. Watch the case: `m` is minutes,
`M` is months.

A bare number is an error, since it could mean anything:

```text
config.jsonc: host.cpu.every: "10" is not a valid duration (time unit needed, for example 10sec or 10ms), expected e.g. "60s" or "10m" at line 3 column 35
```

Every `every` (how often something is checked) must be at least `"1s"`.

### Percentages

A number from 0 to 100 followed by `%`: `"85%"`, `"99.5%"`. Used for CPU,
memory and storage thresholds.

### Temperatures

Degrees Celsius followed by a capital `C`: `"80C"`, `"-5.5C"`. Used for
temperature sensor thresholds.

### Severities

`"critical"`, `"warning"` or `"info"`. Alerts are `"warning"` unless set
otherwise. See [alerts](alerts.md).

## Top-level settings

### `listen` (required)

The address and port of the dashboard, of the endpoint that scheduled jobs
report to, and of [`/status`](alerts.md#status).

```jsonc
"listen": "127.0.0.1:9101"
```

The dashboard has no login, so keep it on `127.0.0.1` unless something in front
of it handles access. See
[exposing the dashboard](installation.md#exposing-the-dashboard).

### `context`

Names this machine in notifications and in [heartbeat](heartbeat.md)
templates. Useful when several machines notify the same place.

```jsonc
"context": "Home server"
```

Defaults to the hostname. It can't be an empty string; leave it out instead.

## Sections

Each section has its own page:

| Key | What | Docs |
| --- | --- | --- |
| `host` | CPU, memory, storage, temperature | [host](host.md) |
| `docker` | Turns container monitoring on | [apps](apps.md) |
| `apps` | Containers, health checks, certificates, services, commands and jobs, grouped per app | [apps](apps.md) |
| `certificates` | How often certificates are checked, and when expiry becomes a warning | [apps](apps.md) |
| `services` | How often services are checked | [apps](apps.md) |
| `notifiers` | Where alerts are sent | [notifiers](notifiers.md) |
| `heartbeat` | A regular request to an outside service | [heartbeat](heartbeat.md) |

Alerts aren't a section of their own. Anything that can alert has an `alert`
key next to it:

```jsonc
"host": {
  "cpu": {
    "every": "10s",
    "alert": { "above": "85%", "for": "10m", "severity": "critical" }
  }
}
```

See [alerts](alerts.md) for every alert setting.

## Defaults

What a setting is when you leave it out:

| Setting | Default |
| --- | --- |
| `context` | the hostname |
| `host.cpu.every`, `host.memory.every` | `"1m"` |
| `host.storage.every` | `"1m"`, for every mount without its own |
| `host.temperature.<sensor>.every` | `"1m"` |
| `docker` | off: containers aren't monitored without this section |
| `docker.every` | `"1m"` |
| `docker.socket` | found automatically, see [the Docker socket](installation.md#the-docker-socket) |
| `certificates.every` | `"6h"` |
| `certificates.warnBefore` | `"14d"` |
| `services.every` | `"1m"` |
| health check `every` | `"1m"` |
| health check `timeout` | `"10s"`, or the check's `every` if that's shorter |
| health check `expectCode` | `200` |
| `heartbeat.every` | `"1m"` |
| `heartbeat.method` | `"GET"` |
| alert `severity` | `"warning"` |

CPU and memory are always watched, even with no `host` section at all.

## Checking the config

`--check` loads the config, validates it, and prints it back with every
default filled in. It doesn't start anything:

```sh
zuschauer --check config.jsonc
```

```text
listen: 127.0.0.1:9101
context: Home server
host.cpu: every 10s
host.memory: every 1m
host.storage /: every 1m
docker: every 5s, socket auto
certificates: every 6h, warn before 14days
services: every 1m
apps.blog: Blog, containers blog-db, certificates example.com
apps.blog.https://example.com/health: GET https://example.com/health every 1m, timeout 10s, expect 200
notifiers.phone: ntfy topic my-server-alerts-7f3k2q at the default server
alert "cpu": CPU, warning, above 85%, for 10m
alert "storage:/": Storage /, warning, above 90%, for 10m
alert "blog/https://example.com/health": Blog https://example.com/health, warning
alert "blog/blog-db": Blog container blog-db, warning
alert "blog/example.com": Blog certificate example.com, warning
```

The last lines list every alert with its ID. To refer to an alert in
`dependsOn`, give it an `id` of your own rather than relying on the generated
one (see [alerts](alerts.md)).

There are two kinds of errors:

- **Syntax and format errors** (a missing quote, a bad duration, an unknown
  key) stop at the first one and point to its line and column.
- **Validation errors** (settings that don't make sense together) are all
  listed at once, each with the path to the key:

  ```text
  config.jsonc: invalid config:
    - apps.blog.containers: containers aren't monitored, add a "docker" section
    - notifiers.phone.topic: expected up to 64 letters, digits, "-" and "_"
  ```

Run `--check` after every edit, before restarting the service.

## A complete example

A home server with one app, alerts to ntfy and a heartbeat:

```jsonc
{
  "listen": "127.0.0.1:9101",
  "context": "Home server",

  "host": {
    "cpu": {
      "every": "10s",
      "alert": { "above": "85%", "for": "10m" }
    },
    "memory": {
      "every": "10s",
      "alert": { "above": "90%", "for": "5m" }
    },
    "storage": {
      // Any existing path works; it reports the filesystem holding it.
      "mounts": {
        "/": { "alert": { "above": "85%", "for": "10m" } },
        "/srv/backups": { "every": "10m", "alert": { "above": "95%", "for": "1h" } }
      }
    },
    // Sensor names come from `zuschauer --sensors`.
    "temperature": {
      "nvme/Composite": {
        "name": "NVMe",
        "alert": { "above": "70C", "for": "10m" }
      }
    }
  },

  "docker": { "every": "30s" },

  "apps": {
    "blog": {
      "name": "Blog",
      "containers": {
        "blog-db": { "alert": { "for": "1m", "id": "blog-db", "severity": "critical" } },
        "blog-web": { "alert": { "for": "1m" } }
      },
      "healthChecks": [
        {
          "url": "https://example.com/health",
          "every": "1m",
          // Stays quiet while the database alert is firing.
          "alert": { "for": "2m", "dependsOn": ["blog-db"] }
        }
      ],
      "certificates": {
        "example.com": { "alert": {} }
      }
    }
  },

  "notifiers": {
    "phone": { "type": "ntfy", "topic": "my-server-alerts-7f3k2q" }
  },

  "heartbeat": {
    "url": "https://hc-ping.com/<uuid>",
    "every": "1m"
  }
}
```
