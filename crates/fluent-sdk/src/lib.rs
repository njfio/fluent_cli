//! Fluent SDK provides strongly typed builders for composing requests to the
//! various engines supported by Fluent.
//!
//! Most users will interact with the [`FluentOpenAIChatRequestBuilder`], but a
//! generic [`FluentRequestBuilder`] is also available when you need full
//! control.

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum::{Display, EnumString};
pub mod openai;

pub mod prelude {
    pub use crate::openai::*;
    pub use crate::{FluentRequest, FluentSdkRequest, KeyValue};
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub data: fluent_core::types::Response,
}
#[async_trait::async_trait]
pub trait FluentSdkRequest: Into<FluentRequest> + Clone {
    fn as_request(&self) -> FluentRequest {
        self.clone().into()
    }
    async fn run(&self) -> anyhow::Result<Response> {
        self.as_request().run().await
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentRequest {
    // The template to use (openai or anthropic)
    pub engine: Option<EngineTemplate>,
    // The credentials to be used on the request
    pub credentials: Option<Vec<KeyValue>>,
    //Overrides for the configuration parameters
    pub overrides: Option<HashMap<String, Value>>,
    // The user prompt to process
    pub request: Option<String>,
    // Parse and display code blocks from the output
    pub parse_code: Option<bool>,
}
impl FluentRequest {
    /// Creates a new [`FluentRequestBuilder`].
    pub fn builder() -> FluentRequestBuilder {
        FluentRequestBuilder::default()
    }
    pub async fn run(&self) -> anyhow::Result<Response> {
        // Convert the implementing type into a FluentRequest
        let request = self.clone();
        // Perform the run logic that was previously in the `run` function
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
        Ok(Response {
            data: fluent_core_response,
        })
    }
}

/// Builder for [`FluentRequest`].
#[derive(Debug, Clone)]
pub struct FluentRequestBuilder {
    request: FluentRequest,
}

impl Default for FluentRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentRequest {
                engine: None,
                credentials: Some(Vec::new()),
                overrides: Some(HashMap::new()),
                request: None,
                parse_code: Some(false),
            },
        }
    }
}

impl FluentRequestBuilder {
    /// Sets the engine template to use.
    pub fn engine(mut self, engine: EngineTemplate) -> Self {
        self.request.engine = Some(engine);
        self
    }

    /// Sets the request string that will be sent to the engine.
    pub fn request(mut self, request: impl Into<String>) -> Self {
        self.request.request = Some(request.into());
        self
    }

    /// Adds a credential key/value pair.
    pub fn credential(mut self, kv: impl Into<KeyValue>) -> Self {
        let entry = self.request.credentials.get_or_insert_with(Vec::new);
        entry.push(kv.into());
        self
    }

    /// Adds a single override parameter.
    pub fn override_param(mut self, key: impl Into<String>, value: Value) -> Self {
        let map = self.request.overrides.get_or_insert_with(HashMap::new);
        map.insert(key.into(), value);
        self
    }

    /// Whether to parse code blocks from the engine output.
    pub fn parse_code(mut self, parse: bool) -> Self {
        self.request.parse_code = Some(parse);
        self
    }

    /// Finalises the builder returning a [`FluentRequest`].
    pub fn build(self) -> anyhow::Result<FluentRequest> {
        if self.request.engine.is_none() {
            return Err(anyhow!("Engine is required"));
        }
        if self.request.request.is_none() {
            return Err(anyhow!("Request is required"));
        }
        Ok(self.request)
    }
}

#[derive(Debug, PartialEq, EnumString, Serialize, Deserialize, Display, Clone)]
pub enum EngineTemplate {
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
