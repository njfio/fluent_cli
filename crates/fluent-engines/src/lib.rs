use log::debug;
use std::str::FromStr;

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
// Plugin imports removed - plugins disabled for security
use anyhow;

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
pub mod replicate;
pub mod secure_plugin_system;
pub mod shared;
pub mod simplified_engine;
pub mod state_store_benchmark;
pub mod streaming_engine;
pub mod universal_base_engine;

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

// Plugin system disabled for security reasons
// TODO: Implement secure plugin architecture with:
// 1. Proper sandboxing and isolation
// 2. Memory safety guarantees
// 3. Plugin signature verification
// 4. Comprehensive error handling
// 5. Security auditing and validation

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
            // Plugin support disabled for security reasons
            debug!(
                "Unknown engine type '{}' - plugins are disabled",
                engine_config.engine
            );
            return Err(anyhow::anyhow!(format!(
                "Unknown engine type: {}",
                engine_config.engine
            )));
        }
    };
    Ok(engine)
}
