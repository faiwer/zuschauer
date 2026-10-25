# Installation

Zuschauer is a single executable. There are no pre-built releases yet, so you
build it from source, copy the binary where you want it, put a config file next
to it and run it as a service.

- [Requirements](#requirements)
- [Building](#building)
- [Cross-compiling for a Linux server](#cross-compiling-for-a-linux-server)

## Requirements

- **To build:** Rust 1.88 or newer, via [rustup](https://rustup.rs).
- **To run:** Linux or macOS. Nothing else: no runtime, no database, no shared
  libraries beyond the system's own (none at all with a musl build).

Optional, depending on what you monitor:

- Docker or Podman, for containers.
- systemd (Linux) or launchd (macOS), for services. Both come with the OS.

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

