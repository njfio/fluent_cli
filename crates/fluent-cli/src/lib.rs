//! Fluent CLI Library
//!
//! This crate provides the main command-line interface for the Fluent CLI system,
//! including agentic capabilities, command handling, pipeline execution,
//! and various utility functions.
//!
//! # Key Modules
//!
//! - [`agentic`] - Autonomous agentic execution capabilities
//! - [`commands`] - Modular command handlers for different CLI operations
//! - [`pipeline_builder`] - Pipeline construction and execution
//! - [`memory`] - Memory management for conversations and context
//! - [`utils`] - Utility functions for text processing and validation
//! - [`cli`] - Core CLI functionality and argument parsing
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_cli::cli::run;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Run the CLI with command line arguments
//! run().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Agentic Mode
//!
//! The CLI supports autonomous agentic execution:
//!
//! ```rust,no_run
//! use fluent_cli::agentic::{AgenticConfig, AgenticExecutor};
//! use fluent_core::config::Config;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = AgenticConfig::new(
//!     "Create a simple game".to_string(),
//!     "agent_config.json".to_string(),
//!     10,
//!     true,
//!     "config.yaml".to_string(),
//! );
//!
//! let executor = AgenticExecutor::new(config);
//! let fluent_config = Config::default();
//! executor.run(&fluent_config).await?;
//! # Ok(())
//! # }
//! ```

pub mod agentic;
pub mod cli;
pub mod commands;
pub mod neo4j_operations;
pub mod pipeline_builder;
pub mod validation;
pub mod memory;
pub mod utils;
pub mod frogger;

// New modular components
pub mod cli_builder;
pub mod engine_factory;
pub mod request_processor;
pub mod response_formatter;

// Refactored CLI modules
pub mod mcp_runner;
pub mod neo4j_runner;

use fluent_engines::create_engine;

// Re-export commonly used functions
pub use utils::{extract_cypher_query, is_valid_cypher, format_as_csv, extract_code};
pub use validation::{validate_engine_name, validate_file_path_secure, parse_key_value_pair};
pub use memory::MemoryManager;

// Re-export main CLI functionality
// CLI functionality moved to main.rs
pub use mcp_runner::{run_mcp_server, run_agentic_mode, run_agent_with_mcp};
pub use neo4j_runner::{get_neo4j_query_llm, generate_cypher_query};
