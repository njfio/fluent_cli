//! Exit codes for CLI operations
//!
//! This module provides standard exit codes that the CLI returns
//! to indicate different types of failures and success.
//!
//! # Standard Exit Codes
//!
//! - `SUCCESS` (0): Operation completed successfully
//! - `GENERAL_ERROR` (1): General/unknown error
//! - `USAGE_ERROR` (2): Incorrect command usage or invalid arguments
//! - `CONFIG_ERROR` (10): Configuration file error (missing, invalid, or malformed)
//! - `NETWORK_ERROR` (4): Network connectivity error
//! - `AUTH_ERROR` (5): Authentication/authorization error (missing or invalid API keys)
//! - `ENGINE_ERROR` (6): Engine-specific error (LLM provider errors)
//! - `VALIDATION_ERROR` (7): Data validation error
//!
//! # Examples
//!
//! ```rust
//! use fluent_cli::exit_codes;
//!
//! // Success case
//! std::process::exit(exit_codes::SUCCESS);
//!
//! // Error case
//! std::process::exit(exit_codes::CONFIG_ERROR);
//! ```

/// Operation completed successfully
pub const SUCCESS: i32 = 0;

/// General or unknown error
pub const GENERAL_ERROR: i32 = 1;

/// Incorrect command usage or invalid arguments
pub const USAGE_ERROR: i32 = 2;

/// Network connectivity error
pub const NETWORK_ERROR: i32 = 4;

/// Authentication or authorization error (missing or invalid API keys)
pub const AUTH_ERROR: i32 = 5;

/// Engine-specific error (LLM provider errors)
pub const ENGINE_ERROR: i32 = 6;

/// Data validation error
pub const VALIDATION_ERROR: i32 = 7;

/// Configuration file error (missing, invalid, or malformed)
/// Using exit code 10 to match existing tests
pub const CONFIG_ERROR: i32 = 10;

/// Maps a CliError to its appropriate exit code
///
/// # Arguments
///
/// * `error` - The CLI error to map
///
/// # Returns
///
/// The appropriate exit code for the error type
///
/// # Examples
///
/// ```rust
/// use fluent_cli::{exit_codes, error::CliError};
///
/// let error = CliError::Config("Missing config file".to_string());
/// let code = exit_codes::error_to_exit_code(&error);
/// assert_eq!(code, exit_codes::CONFIG_ERROR);
/// ```
pub fn error_to_exit_code(error: &crate::error::CliError) -> i32 {
    use crate::error::CliError;

    match error {
        CliError::ArgParse(_) => USAGE_ERROR,
        CliError::Config(_) => CONFIG_ERROR,
        CliError::Engine(_) => ENGINE_ERROR,
        CliError::Network(_) => NETWORK_ERROR,
        CliError::Authentication(_) => AUTH_ERROR,
        CliError::Validation(_) => VALIDATION_ERROR,
        CliError::Unknown(_) => GENERAL_ERROR,
    }
}

/// Maps a general anyhow::Error to its appropriate exit code
///
/// This function examines the error chain to find specific error types
/// and maps them to appropriate exit codes. If no specific error type
/// is found, it returns GENERAL_ERROR.
///
/// # Arguments
///
/// * `error` - The anyhow error to map
///
/// # Returns
///
/// The appropriate exit code for the error
///
/// # Examples
///
/// ```rust
/// use fluent_cli::exit_codes;
/// use anyhow::anyhow;
///
/// let error = anyhow!("Something went wrong");
/// let code = exit_codes::anyhow_error_to_exit_code(&error);
/// assert_eq!(code, exit_codes::GENERAL_ERROR);
/// ```
pub fn anyhow_error_to_exit_code(error: &anyhow::Error) -> i32 {
    use crate::error::CliError;

    // Try to downcast to CliError first
    if let Some(cli_error) = error.downcast_ref::<CliError>() {
        return error_to_exit_code(cli_error);
    }

    // Check error message for specific patterns
    let error_msg = error.to_string().to_lowercase();

    if error_msg.contains("config") || error_msg.contains("configuration") {
        CONFIG_ERROR
    } else if error_msg.contains("api key")
        || error_msg.contains("authentication")
        || error_msg.contains("unauthorized")
    {
        AUTH_ERROR
    } else if error_msg.contains("network")
        || error_msg.contains("connection")
        || error_msg.contains("timeout")
    {
        NETWORK_ERROR
    } else if error_msg.contains("validation") || error_msg.contains("invalid") {
        VALIDATION_ERROR
    } else if error_msg.contains("engine") || error_msg.contains("provider") {
        ENGINE_ERROR
    } else if error_msg.contains("usage") || error_msg.contains("argument") {
        USAGE_ERROR
    } else {
        GENERAL_ERROR
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CliError;

    #[test]
    fn test_error_to_exit_code() {
        assert_eq!(
            error_to_exit_code(&CliError::ArgParse("test".to_string())),
            USAGE_ERROR
        );
        assert_eq!(
            error_to_exit_code(&CliError::Config("test".to_string())),
            CONFIG_ERROR
        );
        assert_eq!(
            error_to_exit_code(&CliError::Engine("test".to_string())),
            ENGINE_ERROR
        );
        assert_eq!(
            error_to_exit_code(&CliError::Network("test".to_string())),
            NETWORK_ERROR
        );
        assert_eq!(
            error_to_exit_code(&CliError::Validation("test".to_string())),
            VALIDATION_ERROR
        );
        assert_eq!(
            error_to_exit_code(&CliError::Unknown("test".to_string())),
            GENERAL_ERROR
        );
    }

    #[test]
    fn test_anyhow_error_to_exit_code_with_cli_error() {
        let error: anyhow::Error = CliError::Config("test".to_string()).into();
        assert_eq!(anyhow_error_to_exit_code(&error), CONFIG_ERROR);

        let error: anyhow::Error = CliError::Network("test".to_string()).into();
        assert_eq!(anyhow_error_to_exit_code(&error), NETWORK_ERROR);
    }

    #[test]
    fn test_anyhow_error_to_exit_code_with_patterns() {
        use anyhow::anyhow;

        let error = anyhow!("Missing API key");
        assert_eq!(anyhow_error_to_exit_code(&error), AUTH_ERROR);

        let error = anyhow!("Network connection failed");
        assert_eq!(anyhow_error_to_exit_code(&error), NETWORK_ERROR);

        let error = anyhow!("Invalid configuration file");
        assert_eq!(anyhow_error_to_exit_code(&error), CONFIG_ERROR);

        let error = anyhow!("Something completely random");
        assert_eq!(anyhow_error_to_exit_code(&error), GENERAL_ERROR);
    }
}
