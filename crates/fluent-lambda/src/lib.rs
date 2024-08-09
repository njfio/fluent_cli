use anyhow::anyhow;
use fluent_core::config::load_engine_config;
use fluent_engines::create_engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, pin::Pin};
use strum::{Display, EnumString};

pub async fn run(request: Request) -> anyhow::Result<Response> {
    let engine_name = request
        .engine
        .map(|t| t.to_string())
        .ok_or_else(|| anyhow!("Engine is required"))?;

    let config_content = include_str!("config.json");

    let overrides = request.overrides.unwrap_or_default();

    let credentials = request
        .credentials
        .unwrap_or_default()
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect::<HashMap<String, String>>();

    let user_prompt = request
        .request
        .ok_or_else(|| anyhow!("Request is required"))?;

    let engine_config = load_engine_config(config_content, &engine_name, &overrides, &credentials)?;

    let max_tokens = engine_config
        .parameters
        .get("max_tokens")
        .and_then(|v| v.as_i64());

    //TODO: Add support for other extended input
    let mut combined_request = user_prompt;

    if let Some(max_tokens) = max_tokens {
        if combined_request.len() > max_tokens as usize {
            combined_request.truncate(max_tokens as usize);
            combined_request += "... [truncated]";
        }
    }

    let engine = create_engine(&engine_config).await?;

    let fluent_request = fluent_core::types::Request {
        flowname: engine_name,
        payload: combined_request,
    };

    let fluent_response = Pin::from(engine.execute(&fluent_request)).await?;

    //TODO: Add support for other extended output

    Ok(Response {
        data: fluent_response,
    })
}

#[derive(Debug, PartialEq, EnumString, Serialize, Deserialize, Display)]
pub enum Template {
    #[strum(ascii_case_insensitive, to_string = "openai")]
    #[serde(alias = "openai")]
    OpenAI,

    #[strum(ascii_case_insensitive, to_string = "anthropic")]
    #[serde(alias = "anthropic")]
    Anthropic,

    #[strum(
        ascii_case_insensitive,
        serialize = "sonnet35",
        to_string = "sonnet3.5"
    )]
    #[serde(alias = "sonnet3.5", alias = "sonnet35")]
    Sonnet35,

    #[strum(
        ascii_case_insensitive,
        serialize = "geminiflash",
        to_string = "gemini-flash"
    )]
    #[serde(alias = "gemini-flash", alias = "geminiflash")]
    GeminiFlash,

    #[strum(
        ascii_case_insensitive,
        serialize = "geminipro",
        to_string = "gemini-pro"
    )]
    #[serde(alias = "gemini-pro", alias = "geminipro")]
    GeminiPro,

    #[strum(ascii_case_insensitive, to_string = "cohere")]
    #[serde(alias = "cohere")]
    Cohere,

    #[strum(
        ascii_case_insensitive,
        serialize = "llama3groq",
        to_string = "llama3-groq"
    )]
    #[serde(alias = "llama3-groq", alias = "llama3groq")]
    Llama3Groq,

    #[strum(
        ascii_case_insensitive,
        serialize = "gemmagroq",
        to_string = "gemma-groq"
    )]
    #[serde(alias = "gemma-groq", alias = "gemmagroq")]
    GemmaGroq,
    #[strum(
        ascii_case_insensitive,
        serialize = "mistralnemo",
        to_string = "mistral-nemo"
    )]
    MistralNemo,
    #[strum(
        ascii_case_insensitive,
        serialize = "mistrallarge2",
        to_string = "mistral-large2"
    )]
    #[serde(alias = "mistral-large2", alias = "mistrallarge2")]
    MistralLarge2,

    #[strum(ascii_case_insensitive, to_string = "perplexity")]
    #[serde(alias = "perplexity")]
    Perplexity,

    #[strum(
        ascii_case_insensitive,
        serialize = "sonnet35chain",
        to_string = "sonnet3.5_chain"
    )]
    #[serde(alias = "sonnet3.5_chain", alias = "sonnet35chain")]
    Sonnet35Chain,

    #[strum(ascii_case_insensitive)]
    OmniAgentWithSearchAndBrowsing,

    #[strum(
        ascii_case_insensitive,
        serialize = "omnichain",
        to_string = "Omni_Chain"
    )]
    #[serde(alias = "Omni_Chain", alias = "omnichain")]
    OmniChain,

    #[strum(
        ascii_case_insensitive,
        serialize = "omnichain2",
        to_string = "Omni_Chain2"
    )]
    #[serde(alias = "Omni_Chain2", alias = "omnichain2")]
    OmniChain2,

    #[strum(
        ascii_case_insensitive,
        serialize = "langflowtest",
        to_string = "langflow_test"
    )]
    #[serde(alias = "langflow_test", alias = "langflowtest")]
    LangFlowTest,

    #[strum(ascii_case_insensitive)]
    MakeLeonardoImagePostRawOutput,

    #[strum(ascii_case_insensitive)]
    StabilityUltraVertical,

    #[strum(ascii_case_insensitive)]
    ImaginePro,

    #[strum(ascii_case_insensitive)]
    LeonardoVertical,

    #[strum(ascii_case_insensitive)]
    DalleVertical,

    #[strum(ascii_case_insensitive)]
    DalleHorizontal,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    // The template to use (openai or anthropic)
    pub engine: Option<Template>,

    // The credentials to be used on the request
    pub credentials: Option<Vec<KeyValue>>,

    //Overrides for the configuration parameters
    pub overrides: Option<HashMap<String, Value>>,

    // The user prompt to process
    pub request: Option<String>,

    // Parse and display code blocks from the output
    pub parse_code: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OverrideValue {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub data: fluent_core::types::Response,
}
