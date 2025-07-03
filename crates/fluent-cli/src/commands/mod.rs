pub mod agent;
pub mod engine;
pub mod mcp;
pub mod neo4j;
/// CLI command modules for organized functionality
pub mod pipeline;

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::ArgMatches;
use fluent_core::config::Config;

/// Trait for CLI command handlers
#[allow(async_fn_in_trait)]
pub trait CommandHandler {
    /// Execute the command with the given arguments and configuration
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()>;
}

/// Command execution result
#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl CommandResult {
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
        }
    }

    pub fn success_with_message(message: String) -> Self {
        Self {
            success: true,
            message: Some(message),
            data: None,
        }
    }

    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message: Some(message),
            data: None,
        }
    }
}
