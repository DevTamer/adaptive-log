# adaptive-log

Auto-detecting logger for Rust: uses [`env_logger`] when your binary runs standalone, and the [systemd journal] when it runs as a managed service. Optional [`tracing`] support behind a feature flag, same detection either way.

```rust
fn main() {
    adaptive_log::init();
    log::info!("ready");
}
```

## How it works

At startup `adaptive_log::init()` checks whether the process is connected to the systemd journal (via the `JOURNAL_STREAM` environment variable that systemd sets automatically). If yes, log records are sent to the journal with proper priority levels and structured fields. If not, `env_logger` takes over and reads the familiar `RUST_LOG` variable.

| Environment | Backend | Output |
|---|---|---|
| Terminal / CI | `env_logger` | Colored, timestamped lines to stderr |
| systemd service | `systemd-journal-logger` | Structured journal entries |

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
adaptive-log = "0.1"
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

The full `RUST_LOG` filter syntax is supported on both paths. Under the journal, the effective maximum level is derived from the filter and per-module filtering is left to the journal's own priority handling.

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
adaptive-log = { version = "0.2", features = ["tracing"] }
tracing = "0.1"
```

```rust
fn main() {
    adaptive_log::init_tracing();
    tracing::info!("ready");
}
```

`init_tracing()` / `init_tracing_with_filter(filter)` build a `tracing_subscriber::Registry`, then attach [`tracing-journald`] under systemd (same journal-connection check as the `log` path) or `tracing_subscriber::fmt` otherwise. `$RUST_LOG` — or the explicit filter string — is parsed by `EnvFilter`, same directive syntax as the `log` path.

This is a separate facade from `init()`/`init_with_filter()`, not a superset of it — pick whichever one matches how your dependencies log. If you depend on crates that use `log` directly, bridge them in with [`tracing-log`]'s `LogTracer`; going the other way, `tracing`'s own `log` feature re-emits its events through the `log` facade.

## License

MIT OR Apache-2.0

[`env_logger`]: https://crates.io/crates/env_logger
[systemd journal]: https://crates.io/crates/systemd-journal-logger
[`tracing`]: https://crates.io/crates/tracing
[`tracing-journald`]: https://crates.io/crates/tracing-journald
[`tracing-log`]: https://crates.io/crates/tracing-log
