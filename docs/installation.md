# Installation

Zuschauer is a single executable. There are no pre-built releases yet, so you
build it from source, copy the binary where you want it, put a config file next
to it and run it as a service.

- [Requirements](#requirements)
- [Building](#building)
- [Cross-compiling for a Linux server](#cross-compiling-for-a-linux-server)
- [Building on FreeBSD](#building-on-freebsd)
- [Checking the config](#checking-the-config)
- [Running as a systemd service (Linux)](#running-as-a-systemd-service-linux)
- [Running as a launchd agent (macOS)](#running-as-a-launchd-agent-macos)
- [Running as an rc.d service (FreeBSD)](#running-as-an-rcd-service-freebsd)
- [Permissions](#permissions)
- [Exposing the dashboard](#exposing-the-dashboard)
- [Updating](#updating)

## Requirements

- **To build:** Rust 1.89 or newer, via [rustup](https://rustup.rs).
- **To run:** Linux, macOS or FreeBSD. Nothing else: no runtime, no database,
  no shared libraries beyond the system's own (none at all with a musl build).

Optional, depending on what you monitor:

- Docker or Podman, for containers. On FreeBSD, jails are containers too.
- systemd (Linux), launchd (macOS) or rc.d (FreeBSD), for services. All of them
  come with the OS.

## Building

```sh
git clone https://github.com/faiwer/zuschauer.git
cd zuschauer
cargo build --release
```

The binary is `target/release/zuschauer`. It's self-contained, so you can copy
it anywhere:

```sh
sudo install -m 755 target/release/zuschauer /usr/local/bin/zuschauer
```

## Cross-compiling for a Linux server

You can build on your laptop and copy the binary to the server. A
[musl](https://musl.libc.org) target gives a fully static binary that runs on
any Linux distribution, whatever its glibc version.

```sh
rustup target add aarch64-unknown-linux-musl   # ARM servers, e.g. Raspberry Pi
rustup target add x86_64-unknown-linux-musl    # Intel and AMD servers
```

On macOS the TLS library needs a C compiler that can target Linux. LLVM from
Homebrew works:

```sh
brew install llvm

CC_aarch64_unknown_linux_musl=/opt/homebrew/opt/llvm/bin/clang \
AR_aarch64_unknown_linux_musl=/opt/homebrew/opt/llvm/bin/llvm-ar \
  cargo build --release --target aarch64-unknown-linux-musl
```

For `x86_64`, use the same command with `x86_64` in place of `aarch64`,
including in the variable names. The binary ends up in
`target/<target>/release/zuschauer`.

## Building on FreeBSD

Build on the FreeBSD machine itself. Rust from packages is recent enough:

```sh
pkg install rust git
git clone https://github.com/faiwer/zuschauer.git
cd zuschauer
cargo build --release
install -m 755 target/release/zuschauer /usr/local/bin/zuschauer
```

Build it on the FreeBSD version it runs on. A binary built on FreeBSD 15 links
`libsys`, which 14 doesn't have, so it won't start there.

## Checking the config

Before starting the service, check the config. `--check` validates it, prints
it with every default filled in, and exits:

```sh
zuschauer --check /etc/zuschauer/config.jsonc
```

```text
listen: 127.0.0.1:9101
context: Home server
host.cpu: every 10s
host.memory: every 1m
docker: every 5s, socket auto
alert "cpu": CPU, warning, above 85%, for 10m
...
```

Mistakes are listed all at once, each with the path to the offending key:

```text
/etc/zuschauer/config.jsonc: invalid config:
  - apps.blog.containers: containers aren't monitored, add a "docker" section
  - notifiers.phone.topic: expected up to 64 letters, digits, "-" and "_"
```

See [configuration](configuration.md) for the format.

## Running as a systemd service (Linux)

Create a user for it, so it doesn't run as root. It doesn't need root to see
other users' containers either (see [permissions](#permissions)):

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin zuschauer
sudo mkdir -p /etc/zuschauer
sudo cp config.jsonc /etc/zuschauer/config.jsonc
```

If the config holds secrets, such as a notifier token, make it readable by
that user only:

```sh
sudo chown zuschauer: /etc/zuschauer/config.jsonc
sudo chmod 600 /etc/zuschauer/config.jsonc
```

Then create `/etc/systemd/system/zuschauer.service`:

```ini
[Unit]
Description=Zuschauer monitoring
After=network-online.target docker.service
Wants=network-online.target

[Service]
User=zuschauer
# Docker only: lets it read Docker's socket for container names. Podman needs
# nothing. Remove it without Docker: systemd refuses to start the service when
# the group doesn't exist.
SupplementaryGroups=docker
ExecStart=/usr/local/bin/zuschauer /etc/zuschauer/config.jsonc
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Start it, and have it start on boot:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now zuschauer
```

Zuschauer logs to stderr, so its messages end up in the journal:

```sh
systemctl status zuschauer
journalctl -u zuschauer -f
```

## Running as a launchd agent (macOS)

On macOS, run it as a launch agent for your own user. Running as you lets it
find the Docker socket in your home directory (OrbStack, Docker Desktop) and
see your user's launchd services.

Put the binary and the config somewhere stable, for example:

```sh
mkdir -p ~/.config/zuschauer
cp target/release/zuschauer /usr/local/bin/zuschauer
cp config.jsonc ~/.config/zuschauer/config.jsonc
```

Create `~/Library/LaunchAgents/com.example.zuschauer.plist`. Replace
`<username>` with yours: launchd doesn't expand `~`.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.example.zuschauer</string>
  <key>ProgramArguments</key>
  <array>
    <string>/usr/local/bin/zuschauer</string>
    <string>/Users/<username>/.config/zuschauer/config.jsonc</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardErrorPath</key>
  <string>/Users/<username>/Library/Logs/zuschauer.log</string>
</dict>
</plist>
```

Load it:

```sh
launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/com.example.zuschauer.plist
```

And to stop it:

```sh
launchctl bootout gui/$(id -u)/com.example.zuschauer
```

## Running as an rc.d service (FreeBSD)

Create a user for it, and put the config where the service expects it:

```sh
pw useradd zuschauer -d /nonexistent -s /usr/sbin/nologin -c "Zuschauer monitoring"
mkdir -p /usr/local/etc/zuschauer
cp config.jsonc /usr/local/etc/zuschauer/config.jsonc
chown zuschauer /usr/local/etc/zuschauer/config.jsonc
chmod 600 /usr/local/etc/zuschauer/config.jsonc
```

With Podman, add the user to the `operator` group, so it can read Podman's
socket for container names:

```sh
pw groupmod operator -m zuschauer
```

Then create `/usr/local/etc/rc.d/zuschauer` and make it executable
(`chmod 555`):

```sh
#!/bin/sh

# PROVIDE: zuschauer
# REQUIRE: LOGIN NETWORKING
# KEYWORD: shutdown

. /etc/rc.subr

name=zuschauer
rcvar=zuschauer_enable

load_rc_config $name

: ${zuschauer_enable:=NO}
: ${zuschauer_user:=zuschauer}
: ${zuschauer_config:=/usr/local/etc/zuschauer/config.jsonc}
: ${zuschauer_log:=/var/log/zuschauer.log}

pidfile=/var/run/${name}.pid
command=/usr/sbin/daemon
command_args="-f -r -P ${pidfile} -u ${zuschauer_user} -o ${zuschauer_log} /usr/local/bin/zuschauer ${zuschauer_config}"
# daemon(8) switches to the user itself; rc.subr would do it too early.
unset zuschauer_user

run_rc_command "$1"
```

[daemon(8)](https://man.freebsd.org/daemon/8) runs zuschauer as that user,
restarts it if it exits, and writes its messages to `/var/log/zuschauer.log`.
Start it, and have it start on boot:

```sh
sysrc zuschauer_enable=YES
service zuschauer start
```

```sh
service zuschauer status
tail -f /var/log/zuschauer.log
```

`zuschauer_user`, `zuschauer_config` and `zuschauer_log` can be changed with
`sysrc` too.

## Permissions

Zuschauer doesn't need root. What it reads, and what that requires:

| What | Reads | Needs |
| --- | --- | --- |
| CPU, memory (Linux) | `/proc/stat`, `/proc/meminfo` | nothing |
| CPU, memory (FreeBSD) | sysctls: `kern.cp_time`, `vm.stats`, `vm.swap_objects`, ZFS's ARC | nothing |
| Storage | the filesystem's own counters, like `df` | nothing |
| Temperature (Linux) | `/sys/class/hwmon` | nothing |
| Temperature (FreeBSD) | the `dev` and `hw.acpi` sysctls | nothing |
| Containers (Linux) | `/sys/fs/cgroup` for CPU and memory, see below for names | nothing, except for Docker's names |
| Containers (macOS) | the Docker socket | running as the user who owns it |
| Containers (FreeBSD) | the jail list and the process table, see below for names | nothing, except for Podman's names |
| Services | `systemctl show` (Linux), `launchctl print` (macOS), `service <name> onestatus` (FreeBSD) | nothing, except on FreeBSD, see below |

### Services on FreeBSD

`service <name> onestatus` reads the service's pid file. A few services keep it
readable by root only, `cron` for example. For an unprivileged user such a
service shows as "can't check it", even though it's running. Running zuschauer
as root avoids that.

### Container names on Linux

Containers are found and measured from cgroups, whoever runs them: rootful or
rootless, any user. Running zuschauer as its own unprivileged user still shows
every container on the machine. Their names come from different places:

- **Podman**, rootful or rootless, any user: from the command line of the
  container's `conmon` process, which any user can read. Containers run by
  Quadlet are named after their systemd unit instead.
- **Docker:** from the Docker socket, which needs membership in the `docker`
  group (see `SupplementaryGroups` above).

A container whose name can't be found is shown by its short ID. That happens
with Docker when the socket isn't readable, and on systems that mount `/proc`
with `hidepid`, which hides other users' processes.

### Container names on FreeBSD

Containers are jails, found with the system's jail list whoever started them:
by hand, Bastille, Podman or anything else. A jail is named as in `jls name`.

Podman names its jails after container IDs, so their names come from Podman's
socket, `/var/run/podman/podman.sock`. It belongs to the `operator` group (see
[above](#running-as-an-rcd-service-freebsd)). Without access, Podman's
containers are shown by their short ID.

### The Docker socket

On Linux the socket is only used for Docker's container names, and on FreeBSD
for Podman's; on macOS everything about containers comes from it. It's found
automatically. Zuschauer tries, in order: `$DOCKER_HOST`,
`/var/run/docker.sock`, Podman's rootful and rootless sockets (FreeBSD's
`/var/run/podman/podman.sock` among them), then OrbStack's and Docker Desktop's
sockets in your home directory. The one it picked is logged at startup:

```text
docker: API at /var/run/docker.sock
```

If it picks the wrong one, or finds none, set it yourself. A configured socket
is used as is, without searching:

```jsonc
"docker": {
  "socket": "/run/user/1000/docker.sock"
}
```

## Exposing the dashboard

The dashboard has no login. Anyone who can reach it sees everything on it,
including your app names and error messages. Keep `listen` on `127.0.0.1` and
reach it in one of these ways:

- **An SSH tunnel.** Nothing to set up on the server:

  ```sh
  ssh -L 9101:127.0.0.1:9101 my-server
  ```

  Then open <http://127.0.0.1:9101>.

- **A VPN** such as Tailscale or WireGuard, with `listen` set to the server's
  VPN address.

- **A reverse proxy** (Caddy, nginx) with HTTP authentication in front of it.

Scheduled jobs report their runs to zuschauer over HTTP, so they need to reach
it too. Jobs on the same machine can use `127.0.0.1`.

## Updating

Build the new binary, replace the old one and restart the service. On Linux, a
running binary can't be overwritten in place ("Text file busy"), but it can be
replaced: copy the new one next to it, then rename it over the old one.

History is kept in memory, so a restart starts the graphs from scratch.
