//! Logging initialization.
//!
//! All logging goes to stderr. Stdout is reserved for MCP protocol
//! messages when using stdio transport.

use crate::cli::LogFormat;
use std::io::IsTerminal;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber.
///
/// - `-q` / `--quiet`: Only errors
/// - (default): Info level
/// - `-v`: Debug level
/// - `-vv` or more: Trace level
/// - `RUST_LOG`: Overrides the above if set
pub fn init(verbose: u8, quiet: bool, format: LogFormat) {
    let level = match (quiet, verbose) {
        (true, _) => "error",
        (_, 0) => "info",
        (_, 1) => "debug",
        (_, _) => "trace",
    };

    let default_filter = format!(
        "{}={},{}={},{}",
        env!("CARGO_PKG_NAME").replace('-', "_"),
        level,
        "{{ project_name }}_core",
        level,
        level
    );

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&default_filter));

    let subscriber = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false);

    match format {
        LogFormat::Auto if std::io::stderr().is_terminal() => {
            subscriber.with_ansi(true).init();
        }
        LogFormat::Auto => {
            subscriber.with_ansi(false).init();
        }
        LogFormat::Pretty => {
            subscriber.with_ansi(true).pretty().init();
        }
        LogFormat::Compact => {
            subscriber.with_ansi(false).compact().init();
        }
        LogFormat::Json => {
            subscriber.json().init();
        }
    }
}
