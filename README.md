# adaptive-log

Auto-detecting logger for Rust: uses [`env_logger`] when your binary runs standalone, and the [systemd journal] when it runs as a managed service. One call, zero configuration.

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

Under `env_logger` the full filter syntax is supported. Under the journal the filter is parsed as a single log level (`error`, `warn`, `info`, `debug`, `trace`).

## Detection

Detection relies on `JOURNAL_STREAM`, which systemd sets for any service whose `StandardOutput` or `StandardError` is `journal` (the default for most unit files). No socket probing, no `/proc` reads.

To simulate journal mode locally:

```sh
JOURNAL_STREAM=8:0 RUST_LOG=debug cargo run
```

## Platform support

`systemd-journal-logger` is Linux-only. On macOS and Windows `adaptive-log` always uses `env_logger`, so the crate compiles and works everywhere without feature flags.

## License

MIT OR Apache-2.0

[`env_logger`]: https://crates.io/crates/env_logger
[systemd journal]: https://crates.io/crates/systemd-journal-logger
