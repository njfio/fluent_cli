//! CLI command modules for organized functionality
//!
//! This module contains all the command handlers for the Fluent CLI,
//! organized into separate modules for different functional areas.
//! Each command module implements the `CommandHandler` trait.

pub mod agent;
pub mod engine;
pub mod mcp;
pub mod neo4j;
pub mod pipeline;
pub mod tools;

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::ArgMatches;
use fluent_core::config::Config;

/// Trait for CLI command handlers
///
/// All command modules must implement this trait to provide
/// a consistent interface for command execution.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_cli::commands::CommandHandler;
/// use clap::ArgMatches;
/// use fluent_core::config::Config;
/// use anyhow::Result;
///
/// struct MyCommand;
///
/// #[async_trait::async_trait]
/// impl CommandHandler for MyCommand {
///     async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
///         println!("Executing my command");
///         Ok(())
///     }
/// }
/// ```
#[allow(async_fn_in_trait)]
pub trait CommandHandler {
    /// Execute the command with the given arguments and configuration
    ///
    /// # Arguments
    ///
    /// * `matches` - Parsed command line arguments
    /// * `config` - Application configuration
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of command execution
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()>;
}

/// Command execution result
///
/// Represents the outcome of a command execution, including
/// success status, optional message, and optional data payload.
///
/// # Examples
///
/// ```rust
/// use fluent_cli::commands::CommandResult;
/// use serde_json::json;
///
/// // Success with no additional data
/// let result = CommandResult::success();
///
/// // Success with a message
/// let result = CommandResult::success_with_message("Operation completed".to_string());
///
/// // Success with structured data
/// let result = CommandResult::success_with_data(json!({"count": 42}));
///
/// // Error result
/// let result = CommandResult::error("Something went wrong".to_string());
/// ```
#[derive(Debug)]
pub struct CommandResult {
    /// Whether the command executed successfully
    pub success: bool,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Optional structured data from the command
    pub data: Option<serde_json::Value>,
}

impl CommandResult {
    /// Create a successful result with no additional data
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
        }
    }

    /// Create a successful result with a message
    ///
    /// # Arguments
    ///
    /// * `message` - Success message to include
    pub fn success_with_message(message: String) -> Self {
        Self {
            success: true,
            message: Some(message),
            data: None,
        }
    }

    /// Create a successful result with structured data
    ///
    /// # Arguments
    ///
    /// * `data` - JSON data to include in the result
    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    /// Create an error result with a message
    ///
    /// # Arguments
    ///
    /// * `message` - Error message describing what went wrong
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            data: None,
        }
    }
}
