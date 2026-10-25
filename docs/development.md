# Development

- [Running it locally](#running-it-locally)
- [Git hooks](#git-hooks)
- [Project layout](#project-layout)
- [Design choices](#design-choices)
- [Web UI](#web-ui)
- [Testing on Linux from a Mac](#testing-on-linux-from-a-mac)

## Running it locally

```sh
cargo run -- config.jsonc
cargo run -- --check config.jsonc
```

The dashboard is then at `http://127.0.0.1:9101`, or whatever `listen` says.
It runs on macOS too, but some collectors only exist on Linux: containers via
cgroups, temperatures via hwmon, and services via systemd.

## Git hooks

[hk](https://hk.jdx.dev) formats the staged snapshot of Rust files on commit and
restores unstaged edits afterward. It is a separate CLI, not a crate dependency.
Install it once, then register the hook once per clone:

```sh
brew install hk
hk install
```

## Project layout

| Module | |
| --- | --- |
| `config` | Reading and validating the config. |
| `collect` | Reading the host, containers, and apps' checks. |
| `store` | The in-memory history behind the graphs. |
| `alerts` | Deciding when alerts fire and resolve. |
| `notify` | Delivering alerts to notifiers. |
| `jobs`, `cron` | Job reports and schedules. |
| `heartbeat` | The outgoing heartbeat. |
| `http` | A small HTTP client, with TLS. |
| `web` | The server and the dashboard. |

## Design choices

- **Few dependencies.** Small things, like the HTTP client and the cron
  parser, are written in-house rather than pulled in.
- **No database.** Everything is kept in memory, and is gone after a restart.
- **No JavaScript build.** Pages are rendered on the server, with a few small
  scripts embedded as they are.
- **The config is checked up front.** Every mistake is reported at startup, at
  once, rather than when the code first reaches it.
- **A small binary.** The release profile optimizes for size.

## Web UI

- `web/components/ui` holds generic pieces, like panels, graphs and icons;
  `web/components/domain` holds zuschauer's own, like the Apps panel.
- CSS, scripts and icons are embedded in the binary.
- Sizes are in rems, colors come from the theme's CSS variables, and spacing
  uses gaps rather than margins.

## Testing on Linux from a Mac

Cross-compile as in the [installation guide](installation.md#cross-compiling-for-a-linux-server),
then run the binary in a Linux container with Docker. For anything that needs
systemd, like rootless Podman or services, use an image that runs systemd as
its init.
