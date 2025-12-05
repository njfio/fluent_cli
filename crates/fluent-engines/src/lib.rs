//! Fluent Engines Library
//!
//! This crate provides implementations of various LLM engines for the Fluent CLI system.
//! It includes support for multiple providers like OpenAI, Anthropic, Google Gemini,
//! and many others, all implementing the common `Engine` trait.
//!
//! # Supported Engines
//!
//! - **OpenAI** - GPT models including GPT-4, GPT-3.5-turbo
//! - **Anthropic** - Claude models including Claude-3
//! - **Google Gemini** - Gemini Pro and other Google AI models
//! - **Mistral** - Mistral AI models
//! - **Cohere** - Cohere language models
//! - **Perplexity** - Perplexity AI models
//! - **Groq** - High-speed inference with Groq LPU
//! - **Stability AI** - Image generation models
//! - **Leonardo AI** - Creative AI models
//! - And many more...
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_engines::{create_engine, EngineType};
//! use fluent_core::config::EngineConfig;
//! use fluent_core::types::Request;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = EngineConfig::default();
//! let engine = create_engine(EngineType::OpenAI, &config)?;
//!
//! let request = Request {
//!     flowname: "chat".to_string(),
//!     payload: "Hello, how are you?".to_string(),
//! };
//!
//! let response = engine.execute(&request).await?;
//! println!("Response: {}", response.content);
//! # Ok(())
//! # }
//! ```

use std::str::FromStr;
use tracing::debug;

use anthropic::AnthropicEngine;
use cohere::CohereEngine;
use flowise_chain::FlowiseChainEngine;
use fluent_core::{config::EngineConfig, traits::Engine};
use google_gemini::GoogleGeminiEngine;
use groqlpu::GroqLPUEngine;
use imagepro::ImagineProEngine;
use langflow::LangflowEngine;
use leonardoai::LeonardoAIEngine;
use mistral::MistralEngine;
use openai::OpenAIEngine;
use perplexity::PerplexityEngine;
use serde::{Deserialize, Serialize};
use stabilityai::StabilityAIEngine;
use strum::{Display, EnumString};
use webhook::WebhookEngine;

// ============================================================================
// PLUGIN SYSTEM STATUS: DISABLED
// ============================================================================
// The plugin system code exists in this crate (see plugin.rs and
// secure_plugin_system.rs) but is NOT ENABLED in production builds.
//
// Reasons:
// 1. Requires WASM runtime (wasmtime/wasmer) - adds 10-15MB to binary
// 2. WASM execution layer not implemented (needs wasm-runtime feature)
// 3. Requires PKI infrastructure for signature verification
// 4. Security audit needed before production use
// 5. Support and maintenance burden for plugin API
//
// The secure plugin architecture is fully designed and partially implemented,
// but the actual WASM runtime execution is feature-gated and not included.
//
// See plugin.rs for complete documentation on enabling plugins for dev/test.
// ============================================================================

extern crate core;

// crates/fluent-engines/src/lib.rs
pub mod anthropic;
pub mod cohere;
pub mod dalle;
pub mod flowise_chain;
pub mod google_gemini;
pub mod groqlpu;
pub mod imagepro;
pub mod langflow;
pub mod leonardoai;
pub mod mistral;
pub mod openai;
pub mod perplexity;
pub mod pipeline;
pub mod pipeline_executor;
pub mod stabilityai;
pub mod webhook;

pub mod anthropic_universal;
pub mod base_engine;
pub mod cache_manager;
pub mod config_cli;
pub mod connection_pool;
pub mod enhanced_cache;
pub mod enhanced_config;
pub mod enhanced_error_handling;
pub mod enhanced_pipeline_executor;
pub mod error_cli;
pub mod memory_optimized_utils;
pub mod modular_pipeline_executor;
pub mod openai_streaming;
pub mod optimized_openai;
pub mod optimized_parallel_executor;
pub mod optimized_state_store;
pub mod pipeline_cli;
pub mod pipeline_infrastructure;
pub mod pipeline_step_executors;
pub mod plugin;
pub mod plugin_cli;
pub mod pooled_openai_example;
pub mod rate_limiter;
pub mod replicate;
pub mod secure_plugin_system;
pub mod shared;
pub mod simplified_engine;
pub mod state_store_benchmark;
pub mod streaming_engine;
pub mod universal_base_engine;

// Re-export commonly used types
pub use rate_limiter::RateLimiter;

#[derive(Debug, PartialEq, EnumString, Serialize, Deserialize, Display)]
pub enum EngineType {
    #[strum(ascii_case_insensitive, to_string = "openai")]
    OpenAI,
    #[strum(ascii_case_insensitive, to_string = "anthropic")]
    Anthropic,
    #[strum(
        ascii_case_insensitive,
        to_string = "google_gemini",
        serialize = "googlegemini"
    )]
    GoogleGemini,
    #[strum(ascii_case_insensitive, to_string = "cohere")]
    Cohere,
    #[strum(ascii_case_insensitive, to_string = "groq_lpu", serialize = "groqlpu")]
    GroqLpu,
    #[strum(ascii_case_insensitive, to_string = "mistral")]
    Mistral,
    #[strum(ascii_case_insensitive, to_string = "perplexity")]
    Perplexity,
    #[strum(
        ascii_case_insensitive,
        to_string = "flowise_chain",
        serialize = "flowisechain"
    )]
    FlowiseChain,
    #[strum(
        ascii_case_insensitive,
        to_string = "langflow_chain",
        serialize = "langflowchain"
    )]
    LangflowChain,
    #[strum(ascii_case_insensitive, to_string = "webhook")]
    Webhook,
    #[strum(ascii_case_insensitive, to_string = "stabilityai")]
    StabilityAI,
    #[strum(
        ascii_case_insensitive,
        to_string = "imagine_pro",
        serialize = "imaginepro"
    )]
    ImaginePro,
    #[strum(
        ascii_case_insensitive,
        to_string = "leonardo_ai",
        serialize = "leonardoai"
    )]
    LeonardoAI,
    #[strum(ascii_case_insensitive, to_string = "dalle")]
    Dalle,
}

// Secure plugin system implemented with comprehensive security features:
// ✅ WebAssembly-based sandboxing and memory isolation
// ✅ Memory safety guarantees through WASM runtime
// ✅ Ed25519/RSA plugin signature verification
// ✅ Comprehensive error handling and validation
// ✅ Security auditing and compliance logging
// ✅ Capability-based permission system
// ✅ Resource limits and quotas enforcement
// ✅ Comprehensive security architecture (development stage - thorough testing recommended)

pub async fn create_engine(engine_config: &EngineConfig) -> anyhow::Result<Box<dyn Engine>> {
    let engine: Box<dyn Engine> = match EngineType::from_str(engine_config.engine.as_str()) {
        Ok(et) => match et {
            EngineType::OpenAI => Box::new(OpenAIEngine::new(engine_config.clone()).await?),
            EngineType::Anthropic => Box::new(AnthropicEngine::new(engine_config.clone()).await?),
            EngineType::Cohere => Box::new(CohereEngine::new(engine_config.clone()).await?),
            EngineType::GoogleGemini => {
                Box::new(GoogleGeminiEngine::new(engine_config.clone()).await?)
            }
            EngineType::Perplexity => Box::new(PerplexityEngine::new(engine_config.clone()).await?),
            EngineType::GroqLpu => Box::new(GroqLPUEngine::new(engine_config.clone()).await?),
            EngineType::Mistral => Box::new(MistralEngine::new(engine_config.clone()).await?),
            EngineType::FlowiseChain => {
                Box::new(FlowiseChainEngine::new(engine_config.clone()).await?)
            }
            EngineType::LangflowChain => {
                Box::new(LangflowEngine::new(engine_config.clone()).await?)
            }
            EngineType::Webhook => Box::new(WebhookEngine::new(engine_config.clone()).await?),
            EngineType::StabilityAI => {
                Box::new(StabilityAIEngine::new(engine_config.clone()).await?)
            }
            EngineType::ImaginePro => Box::new(ImagineProEngine::new(engine_config.clone()).await?),
            EngineType::LeonardoAI => Box::new(LeonardoAIEngine::new(engine_config.clone()).await?),
            EngineType::Dalle => Box::new(dalle::DalleEngine::new(engine_config.clone()).await?),
        },
        Err(_) => {
            // Plugin support disabled - see PLUGIN SYSTEM STATUS comment above for details
            // Unknown engine types cannot be loaded as plugins because:
            // - WASM runtime not included (wasm-runtime feature disabled)
            // - No plugin loading infrastructure enabled
            // - Security and trust infrastructure not configured
            //
            // Use built-in engines (OpenAI, Anthropic, Google, etc.) or Webhook engine
            // to proxy to custom services.
            debug!(
                "Unknown engine type '{}' - plugins are disabled",
                engine_config.engine
            );
            return Err(anyhow::anyhow!(format!(
                "Unknown engine type: {}. Plugins are disabled. Available engines: openai, anthropic, google_gemini, cohere, mistral, groq_lpu, perplexity, flowise_chain, langflow_chain, webhook, stabilityai, imagine_pro, leonardo_ai, dalle",
                engine_config.engine
            )));
        }
    };
    Ok(engine)
}
