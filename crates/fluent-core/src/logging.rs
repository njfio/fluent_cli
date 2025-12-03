//! Centralized logging configuration for the Fluent CLI system
//!
//! This module provides consistent logging initialization across all binaries
//! using the tracing-subscriber framework. It supports both human-readable
//! and JSON-formatted output for different deployment scenarios.
//!
//! # Logging Configuration
//!
//! Set log level via `RUST_LOG` environment variable:
//! - `RUST_LOG=debug` - Enable debug logging
//! - `RUST_LOG=fluent_cli=debug,fluent_agent=info` - Per-crate control
//! - `RUST_LOG=warn` - Only warnings and errors
//!
//! Set log format via `FLUENT_LOG_FORMAT` environment variable:
//! - `FLUENT_LOG_FORMAT=json` - JSON formatted logs
//! - `FLUENT_LOG_FORMAT=human` - Human-readable logs (default)
//!
//! Or use CLI flags:
//! - `--json-logs` - Enable JSON logging
//! - `--human-logs` - Enable human-readable logging
//!
//! Set verbosity via flags or environment:
//! - `--verbose` or `FLUENT_VERBOSE=1` - Enable verbose logging
//! - `--quiet` or `FLUENT_QUIET=1` - Suppress non-error output
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_core::logging;
//!
//! fn main() {
//!     // Initialize with default settings (human-readable)
//!     logging::init_logging();
//!
//!     // Or initialize with JSON output
//!     logging::init_json_logging();
//!
//!     // Now use tracing macros
//!     tracing::info!("Application started");
//!     tracing::debug!(key = "value", "Debug message");
//! }
//! ```

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize logging with human-readable output
///
/// This function sets up tracing with a human-friendly format suitable
/// for development and interactive CLI usage. Log levels are controlled
/// via the `RUST_LOG` environment variable, defaulting to "info".
///
/// This function will silently ignore errors if logging is already initialized.
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .try_init();
}

/// Initialize logging with JSON output
///
/// This function sets up tracing with JSON-formatted output suitable for
/// production deployments, log aggregation systems, and structured logging.
/// Each log entry includes fields like timestamp, level, target, and message.
///
/// This function will silently ignore errors if logging is already initialized.
pub fn init_json_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let _ = tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(filter)
        .try_init();
}

/// Initialize logging based on environment variables and CLI flags
///
/// This function provides intelligent logging initialization that respects
/// multiple configuration sources in the following priority order:
/// 1. CLI flags (--json-logs, --human-logs)
/// 2. FLUENT_LOG_FORMAT environment variable
/// 3. Default (human-readable)
///
/// # Arguments
///
/// * `use_json` - Optional flag to force JSON output. If None, environment
///   variables will be checked.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::logging;
///
/// // Auto-detect from environment
/// logging::init_logging_with_options(None);
///
/// // Force JSON output
/// logging::init_logging_with_options(Some(true));
///
/// // Force human-readable output
/// logging::init_logging_with_options(Some(false));
/// ```
pub fn init_logging_with_options(use_json: Option<bool>) {
    let should_use_json = use_json.unwrap_or_else(|| {
        std::env::var("FLUENT_LOG_FORMAT")
            .map(|v| v.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
    });

    if should_use_json {
        init_json_logging();
    } else {
        init_logging();
    }
}

/// Initialize logging for CLI applications with request ID support
///
/// This is a convenience function that combines logging initialization
/// with request ID generation and environment setup. It's designed for
/// CLI applications that need structured logging with request correlation.
///
/// # Returns
///
/// Returns the generated request ID as a String
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::logging;
///
/// fn main() {
///     let request_id = logging::init_cli_logging();
///     tracing::info!(request_id = %request_id, "Application started");
/// }
/// ```
pub fn init_cli_logging() -> String {
    // Check for CLI flags in argv
    let args: Vec<String> = std::env::args().collect();
    let use_json = if args.iter().any(|a| a == "--json-logs") {
        Some(true)
    } else if args.iter().any(|a| a == "--human-logs") {
        Some(false)
    } else {
        None
    };

    // Initialize logging
    init_logging_with_options(use_json);

    // Generate and store request ID
    let request_id = uuid::Uuid::new_v4().to_string();
    std::env::set_var("FLUENT_REQUEST_ID", &request_id);

    request_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging_does_not_panic() {
        // Multiple calls should not panic
        init_logging();
        init_logging();
    }

    #[test]
    fn test_init_json_logging_does_not_panic() {
        init_json_logging();
        init_json_logging();
    }

    #[test]
    fn test_init_with_options() {
        init_logging_with_options(Some(true));
        init_logging_with_options(Some(false));
        init_logging_with_options(None);
    }

    #[test]
    fn test_cli_logging_generates_request_id() {
        let request_id = init_cli_logging();
        assert!(!request_id.is_empty());
        assert_eq!(
            std::env::var("FLUENT_REQUEST_ID").unwrap(),
            request_id
        );
    }
}
