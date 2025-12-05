//! Fluent CLI Library
//!
//! This crate provides the main command-line interface for the Fluent CLI system,
//! including agentic capabilities, command handling, pipeline execution,
//! and various utility functions.
//!
//! # Key Modules
//!
//! - [`agentic`] - Autonomous agentic execution capabilities
//! - [`agent_control`] - Human-in-the-loop control channel for agent collaboration
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
pub mod code_validation;
pub mod commands;
pub mod memory;
pub mod neo4j_operations;
pub mod pipeline_builder;
pub mod tui;
pub mod validation;
// pub mod frogger; // Removed frogger module as it doesn't exist

// New modular components
pub mod cli_builder;
pub mod engine_factory;
pub mod request_processor;
pub mod response_formatter;

// Refactored CLI modules
pub mod cli;
pub mod error;
pub mod exit_codes;
pub mod mcp_runner;
pub mod neo4j_runner;
pub mod utils; // Added utils module

// Re-export commonly used functions
// Updated to use the local utils module instead of trying to import from a non-existent path
pub use code_validation::{validate_generated_code, ValidationResult};
pub use fluent_engines::create_engine;
pub use memory::MemoryManager;
pub use utils::{extract_code, extract_cypher_query, format_as_csv, is_valid_cypher};
pub use validation::{parse_key_value_pair, validate_engine_name, validate_file_path_secure};

// Re-export main CLI functionality
pub use cli::{run, run_modular};
pub use cli_builder::build_cli;
// Removed print_response as it doesn't exist in the cli module

// Re-export MCP runner functions
pub use mcp_runner::{run_agent_with_mcp, run_agentic_mode, run_mcp_server};
pub use neo4j_runner::{generate_cypher_query, get_neo4j_query_llm};
