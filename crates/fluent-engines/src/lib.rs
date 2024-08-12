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

pub mod replicate;

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

pub async fn create_engine(engine_config: &EngineConfig) -> anyhow::Result<Box<dyn Engine>> {
    let engine: Box<dyn Engine> = match EngineType::from_str(engine_config.engine.as_str())? {
        EngineType::OpenAI => Box::new(OpenAIEngine::new(engine_config.clone()).await?),
        EngineType::Anthropic => Box::new(AnthropicEngine::new(engine_config.clone()).await?),
        EngineType::Cohere => Box::new(CohereEngine::new(engine_config.clone()).await?),
        EngineType::GoogleGemini => Box::new(GoogleGeminiEngine::new(engine_config.clone()).await?),
        EngineType::Perplexity => Box::new(PerplexityEngine::new(engine_config.clone()).await?),
        EngineType::GroqLpu => Box::new(GroqLPUEngine::new(engine_config.clone()).await?),
        EngineType::Mistral => Box::new(MistralEngine::new(engine_config.clone()).await?),
        EngineType::FlowiseChain => Box::new(FlowiseChainEngine::new(engine_config.clone()).await?),
        EngineType::LangflowChain => Box::new(LangflowEngine::new(engine_config.clone()).await?),
        EngineType::Webhook => Box::new(WebhookEngine::new(engine_config.clone()).await?),
        EngineType::StabilityAI => Box::new(StabilityAIEngine::new(engine_config.clone()).await?),
        EngineType::ImaginePro => Box::new(ImagineProEngine::new(engine_config.clone()).await?),
        EngineType::LeonardoAI => Box::new(LeonardoAIEngine::new(engine_config.clone()).await?),
        EngineType::Dalle => Box::new(dalle::DalleEngine::new(engine_config.clone()).await?),
    };
    Ok(engine)
}
