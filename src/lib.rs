//! A thin glue crate that auto-detects whether the process is running under
//! systemd and routes log output accordingly.
//!
//! - **Under systemd**: structured output to the systemd journal via
//!   `systemd_journal_logger` (Linux only). No ANSI noise; severity maps to
//!   journal priority levels.
//! - **Standalone / non-Linux**: colored, `RUST_LOG`-driven output to stderr
//!   via [`env_logger`].
//!
//! # Usage
//!
//! ```rust,no_run
//! // One-liner init
//! adaptive_log::init();
//!
//! // With an explicit filter string (same syntax as RUST_LOG)
//! adaptive_log::init_with_filter("info");
//! ```
//!
//! # `tracing` support
//!
//! Enable the `tracing` feature to get the same auto-detection wired into a
//! [`tracing`] [`Registry`](tracing_subscriber::Registry) instead of the `log`
//! facade: `adaptive_log::init_tracing()` installs `tracing-journald` under
//! systemd, or `tracing_subscriber::fmt` otherwise. The two facades are
//! independent, so pick one `init*` function, not both.

pub use log;

#[cfg(feature = "tracing")]
pub use tracing;

#[cfg(target_os = "linux")]
use log::LevelFilter;

/// Initialises the global logger.
///
/// - On Linux, if the process is connected to the systemd journal (detected via
///   `$JOURNAL_STREAM`), installs the journal logger with max level `Trace` so
///   the journal handles priority filtering itself.
/// - Falls back to [`env_logger`] (reads `$RUST_LOG`) in all other cases,
///   including non-Linux platforms, processes not started by systemd, and
///   environments where the journal socket is unavailable (e.g. containers).
///
/// # Panics
///
/// Panics if a global logger has already been installed.
pub fn init() {
    try_init().expect("adaptive_log::init called after a logger was already installed");
}

/// Initialises the global logger with an explicit filter string.
///
/// The `filter` argument uses the same syntax as `RUST_LOG`
/// (e.g. `"info"`, `"myapp=debug,warn"`).
///
/// - On Linux with a journal connection, the max log level is derived from
///   `filter` and set via `log::set_max_level`; the journal applies no further
///   filtering.
/// - Otherwise, `filter` is passed directly to [`env_logger::Builder::parse_filters`].
///
/// # Panics
///
/// Panics if a global logger has already been installed.
pub fn init_with_filter(filter: &str) {
    try_init_with_filter(filter)
        .expect("adaptive_log::init_with_filter called after a logger was already installed");
}

/// Initialises a global `tracing` subscriber (requires the `tracing` feature).
///
/// Uses the same detection as [`init`]: on Linux, if the process is connected
/// to the systemd journal, installs `tracing-journald` as the output layer
/// (Linux only, so the link doesn't resolve in docs built elsewhere).
/// Otherwise falls back to `tracing_subscriber::fmt`.
/// Verbosity in both cases comes from `$RUST_LOG`, parsed by
/// [`tracing_subscriber::EnvFilter`], defaulting to `info` if unset or invalid.
///
/// This installs a `tracing` [`Registry`](tracing_subscriber::Registry), not a
/// `log` logger, and it's independent of [`init`]/[`init_with_filter`]. Call one
/// or the other, not both.
///
/// # Panics
///
/// Panics if a global subscriber has already been installed.
#[cfg(feature = "tracing")]
pub fn init_tracing() {
    try_init_tracing(None)
        .expect("adaptive_log::init_tracing called after a global subscriber was already set");
}

/// Initialises a global `tracing` subscriber with an explicit filter string
/// (requires the `tracing` feature).
///
/// The `filter` argument uses the same directive syntax as `$RUST_LOG`
/// (e.g. `"info"`, `"my_crate=debug,warn"`), passed straight to
/// [`tracing_subscriber::EnvFilter`].
///
/// # Panics
///
/// Panics if a global subscriber has already been installed.
#[cfg(feature = "tracing")]
pub fn init_tracing_with_filter(filter: &str) {
    try_init_tracing(Some(filter))
        .expect("adaptive_log::init_tracing_with_filter called after a global subscriber was already set");
}

#[cfg(feature = "tracing")]
fn try_init_tracing(filter: Option<&str>) -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::{prelude::*, EnvFilter, Registry};

    let env_filter = match filter {
        Some(f) => EnvFilter::new(f),
        None => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    };

    let registry = Registry::default().with(env_filter);

    // Reuses the exact same detection as the `log` path: a journal socket
    // being reachable is not enough (it's reachable on any systemd-based
    // desktop). This checks that *this process's* stderr was actually wired
    // to the journal by systemd.
    #[cfg(target_os = "linux")]
    if systemd_journal_logger::connected_to_journal() {
        if let Ok(journald) = tracing_journald::layer() {
            return registry.with(journald).try_init();
        }
    }

    registry.with(tracing_subscriber::fmt::layer()).try_init()
}

// ── internal ──────────────────────────────────────────────────────────────────

fn try_init() -> Result<(), log::SetLoggerError> {
    #[cfg(target_os = "linux")]
    if try_install_journal(LevelFilter::Trace) {
        return Ok(());
    }

    env_logger::builder().try_init()
}

fn try_init_with_filter(filter: &str) -> Result<(), log::SetLoggerError> {
    #[cfg(target_os = "linux")]
    {
        let level = max_level_from_filter(filter);
        if try_install_journal(level) {
            return Ok(());
        }
    }

    env_logger::Builder::new().parse_filters(filter).try_init()
}

/// Derives the maximum `LevelFilter` from a RUST_LOG-style filter string
/// without installing any logger.
#[cfg(target_os = "linux")]
fn max_level_from_filter(filter: &str) -> LevelFilter {
    let mut builder = env_logger::Builder::new();
    builder.parse_filters(filter);
    builder.build().filter()
}

/// Attempts to connect to and install the systemd journal logger.
///
/// Returns `true` on success. Returns `false` (falling back to env_logger) if:
/// - the process is not connected to the journal (`$JOURNAL_STREAM` absent/mismatched)
/// - the journal socket is unavailable (e.g. inside a container without systemd)
/// - a logger was already installed concurrently
#[cfg(target_os = "linux")]
fn try_install_journal(max_level: LevelFilter) -> bool {
    use systemd_journal_logger::{connected_to_journal, JournalLog};

    if !connected_to_journal() {
        return false;
    }

    let logger = match JournalLog::new() {
        Ok(l) => l,
        Err(_) => return false,
    };

    // install() calls log::set_boxed_logger but does NOT call set_max_level.
    match logger.install() {
        Ok(()) => {
            log::set_max_level(max_level);
            true
        }
        Err(_) => false,
    }
}
