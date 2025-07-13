//! Fluent Core Library
//!
//! This crate provides the core functionality for the Fluent CLI system,
//! including configuration management, error handling, authentication,
//! caching, and fundamental types and traits.
//!
//! # Key Modules
//!
//! - [`config`] - Configuration management and loading
//! - [`types`] - Core data types for requests, responses, and usage tracking
//! - [`traits`] - Fundamental traits for engines and file handling
//! - [`error`] - Error types and handling utilities
//! - [`auth`] - Authentication and credential management
//! - [`cache`] - Caching functionality for performance optimization
//! - [`neo4j`] - Neo4j graph database integration
//! - [`utils`] - General utility functions
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_core::config::load_config;
//! use fluent_core::types::Request;
//! use std::collections::HashMap;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Load configuration
//! let config = load_config("config.yaml", "openai", &HashMap::new())?;
//!
//! // Create a request
//! let request = Request {
//!     flowname: "chat".to_string(),
//!     payload: "Hello, world!".to_string(),
//! };
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod cache;
pub mod centralized_config;
pub mod config;
pub mod cost_calculator;
pub mod error;
pub mod input_validator;
pub mod poison_recovery;
pub mod lock_timeout;
pub mod deadlock_prevention;
pub mod memory_utils;
pub mod neo4j;
pub mod neo4j_client;
pub mod output;
pub mod output_processor;
pub mod spinner_configuration;
pub mod traits;
pub mod types;
pub mod utils;
mod voyageai_client;
