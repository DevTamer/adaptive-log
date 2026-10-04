# adaptive-log

Auto-detecting logger for Rust: uses [`env_logger`] when your binary runs standalone, and the [systemd journal] when it runs as a managed service. Optional [`tracing`] support behind a feature flag, same detection either way.

```rust
fn main() {
    adaptive_log::init();
    log::info!("ready");
}
```

## How it works

At startup `adaptive_log::init()` checks whether the process is connected to the systemd journal (via the `JOURNAL_STREAM` environment variable that systemd sets automatically). If yes, log records are sent to the journal with proper priority levels and structured fields. If not, `env_logger` takes over. Either way, verbosity is controlled by the familiar `RUST_LOG` variable.

| Environment | Backend | Output |
|---|---|---|
| Terminal / CI | `env_logger` | Colored, timestamped lines to stderr |
| systemd service | `systemd-journal-logger` | Structured journal entries |

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
adaptive-log = "0.3"
log = "0.4"
```

### One-liner init

```rust
adaptive_log::init();
```

### Init with explicit filter

```rust
adaptive_log::init_with_filter("my_crate=debug,warn");
```

The explicit filter replaces `RUST_LOG`, which is then not read.

## Filtering

Both backends use env_logger's filter syntax in full, including per-module directives such as `my_crate=debug,warn`. If `RUST_LOG` is unset, the filter defaults to `info`. Note that this differs from plain `env_logger`, which defaults to `error`; set `RUST_LOG=error` if you want that behaviour.

Under systemd, set `RUST_LOG` in the unit file:

```ini
[Service]
Environment=RUST_LOG=my_crate=debug,info
```

Or on NixOS:

```nix
systemd.services.my-service.environment.RUST_LOG = "my_crate=debug,info";
```

## Detection

Detection relies on `JOURNAL_STREAM`, which systemd sets for any service whose `StandardOutput` or `StandardError` is `journal` (the default for most unit files). The value is verified against the actual stderr file descriptor, so it can't be spoofed by simply exporting the variable.

If the journal socket turns out to be unavailable (e.g. inside a container without systemd), the crate silently falls back to `env_logger` instead of failing.

To test the journal path on a Linux machine:

```sh
cargo build --example basic
systemd-run --user --wait ./target/debug/examples/basic
journalctl --user -n 10
```

## Platform support

`systemd-journal-logger` is Linux-only. On macOS and Windows `adaptive-log` always uses `env_logger`, so the crate compiles and works everywhere without feature flags.

## `tracing` support

Enable the `tracing` feature for the same auto-detection wired into [`tracing`] instead of the `log` facade:

```toml
[dependencies]
adaptive-log = { version = "0.3", features = ["tracing"] }
tracing = "0.1"
```

```rust
fn main() {
    adaptive_log::init_tracing();
    tracing::info!("ready");
}
```

`init_tracing()` / `init_tracing_with_filter(filter)` build a `tracing_subscriber::Registry`, then attach [`tracing-journald`] under systemd (same journal-connection check as the `log` path) or `tracing_subscriber::fmt` otherwise. `$RUST_LOG`, or the explicit filter string, is parsed by `EnvFilter` using the same directive syntax and the same `info` default as the `log` path.

The two `init` paths install different global backends, so call one or the other. Crates that log with `log` macros show up under `init_tracing()` if you also call [`tracing-log`]'s `LogTracer::init()`. Crates that use `tracing` show up under `init()` if you enable the `log` feature on `tracing`, which forwards their events to the `log` facade when no tracing subscriber is installed.

## License

MIT OR Apache-2.0

[`env_logger`]: https://crates.io/crates/env_logger
[systemd journal]: https://crates.io/crates/systemd-journal-logger
[`tracing`]: https://crates.io/crates/tracing
[`tracing-journald`]: https://crates.io/crates/tracing-journald
[`tracing-log`]: https://crates.io/crates/tracing-log
