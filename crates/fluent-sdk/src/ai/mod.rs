use std::collections::HashMap;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::{Display, EnumString};

pub mod cohere;
pub mod flowise_sonnet_chain;
pub mod gemini_flash;
pub mod gemini_pro;
pub mod gemma_groq;
pub mod imaginepro;
pub mod leonardo;
pub mod llama3_groq;
pub mod mistral_large2;
pub mod mistral_nemo;
pub mod openai;
pub mod openai_dalle;
pub mod perplexity;
pub mod sonnet35;
pub mod stability_ultravertical;

pub fn get_credential_from_env_or_value(
    key: &str,
    value: Option<String>,
) -> anyhow::Result<KeyValue> {
    match value.or_else(|| std::env::var(key).ok()) {
        Some(value) => Ok(KeyValue::new(key, &value)),
        None => Err(anyhow!("Credential {} is required", key)),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FluentResponse {
    pub data: fluent_core::types::Response,
}
#[async_trait::async_trait]
pub trait FluentSdkRequest: Send + Sync {
    fn as_request(&self) -> FluentRequest;

    async fn run(&self) -> anyhow::Result<FluentResponse> {
        self.as_request().run().await
    }

    fn clone_box(&self) -> Box<dyn FluentSdkRequest>;
}

impl<T> FluentSdkRequest for T
where
    T: 'static + Into<FluentRequest> + Clone + Send + Sync,
{
    fn as_request(&self) -> FluentRequest {
        self.clone().into()
    }

    fn clone_box(&self) -> Box<dyn FluentSdkRequest> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn FluentSdkRequest> {
    fn clone(&self) -> Box<dyn FluentSdkRequest> {
        self.clone_box()
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentRequest {
    // The template to use (openai or anthropic)
    pub engine: Option<EngineName>,
    // The credentials to be used on the request
    pub credentials: Option<Vec<KeyValue>>,
    //Overrides for the configuration parameters
    pub overrides: Option<HashMap<String, Value>>,
    // The user prompt to process
    pub request: Option<String>,
}
impl FluentRequest {
    pub async fn run(&self) -> anyhow::Result<FluentResponse> {
        // Convert the implementing type into a FluentRequest
        let request = self.clone();
        // Perform the run logic that was previously in the `run` function
        let engine_name = request
            .engine
            .map(|t| t.to_string())
            .ok_or_else(|| anyhow!("Engine is required"))?;
        let config_content = include_str!("../config.json");
        let overrides = request.overrides.unwrap_or_default();
        let credentials = request
            .credentials
            .unwrap_or_default()
            .into_iter()
            .map(|kv| (kv.key, kv.value))
            .collect::<std::collections::HashMap<String, String>>();
        let user_prompt = request
            .request
            .ok_or_else(|| anyhow!("Request is required"))?;
        let engine_config = fluent_core::config::load_engine_config(
            config_content,
            &engine_name,
            &overrides,
            &credentials,
        )?;
        let max_tokens = engine_config
            .parameters
            .get("max_tokens")
            .and_then(|v| v.as_i64());
        // Prepare the combined request
        let mut combined_request = user_prompt;
        if let Some(max_tokens) = max_tokens {
            if combined_request.len() > max_tokens as usize {
                combined_request.truncate(max_tokens as usize);
                combined_request += "... [truncated]";
            }
        }
        let engine = fluent_engines::create_engine(&engine_config).await?;
        let fluent_core_request = fluent_core::types::Request {
            flowname: engine_name,
            payload: combined_request,
        };
        let fluent_core_response =
            std::pin::Pin::from(engine.execute(&fluent_core_request)).await?;
        Ok(FluentResponse {
            data: fluent_core_response,
        })
    }
}

#[derive(Debug, PartialEq, EnumString, Serialize, Deserialize, Display, Clone)]
pub enum EngineName {
    #[strum(ascii_case_insensitive, to_string = "openai-chat-completions")]
    #[serde(alias = "openai-chat-completions", alias = "openai")]
    OpenAIChatCompletions,

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
    FlowiseSonnet35Chain,

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
    #[serde(alias = "leonardo", alias = "leonardo")]
    Leonardo,

    #[strum(ascii_case_insensitive, to_string = "openai-dalle")]
    #[serde(alias = "openai-dalle")]
    OpenAiDalle,

    #[strum(ascii_case_insensitive)]
    DalleHorizontal,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}
impl KeyValue {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

impl<K: Into<String>, V: Into<String>> From<(K, V)> for KeyValue {
    fn from(kv: (K, V)) -> Self {
        Self {
            key: kv.0.into(),
            value: kv.1.into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OverrideValue {
    pub key: String,
    pub value: Value,
}
