# Changelog

## 0.3.0

### Changed

- Under systemd, `init()` now respects `RUST_LOG`. Previously the journal logger was installed at `trace` and `RUST_LOG` was ignored, so `Environment=RUST_LOG=...` in a unit file had no effect.
- Per-module directives (e.g. `my_crate=debug,warn`) now apply to the journal for both `init()` and `init_with_filter()`. Previously only the overall maximum level was kept.
- If `RUST_LOG` is unset, the filter defaults to `info` on every path. Previously it was `trace` on the journal and `error` (env_logger's default) in the terminal. Set `RUST_LOG=error` to get the old terminal behaviour.

## 0.2.1

- Build docs.rs with all features so the `tracing` API is documented.

## 0.2.0

- Add an optional `tracing` feature with `init_tracing()` and `init_tracing_with_filter()`.

## 0.1.0

- Initial release.
