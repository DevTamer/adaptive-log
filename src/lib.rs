//! A thin glue crate that auto-detects whether the process is running under
//! systemd and routes log output accordingly.
//!
//! - **Under systemd**: structured output to the systemd journal via
//!   [`systemd_journal_logger`] (Linux only). No ANSI noise; severity maps to
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

pub use log;

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
