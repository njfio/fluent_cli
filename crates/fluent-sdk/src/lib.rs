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

/// Explicit error types for SDK operations.
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("Invalid configuration: {field} - {message}")]
    InvalidConfig { field: String, message: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid override: {key} - {reason}")]
    InvalidOverride { key: String, reason: String },

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

pub mod prelude {
    pub use crate::openai::*;
    pub use crate::{FluentRequest, FluentSdkRequest, KeyValue, SdkError};
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

    /// Validates the current builder state.
    pub fn validate(&self) -> Result<(), SdkError> {
        // Validate required fields
        if self.request.engine.is_none() {
            return Err(SdkError::MissingField("engine".to_string()));
        }

        if let Some(ref req) = self.request.request {
            if req.is_empty() {
                return Err(SdkError::MissingField("request".to_string()));
            }
        } else {
            return Err(SdkError::MissingField("request".to_string()));
        }

        // Validate overrides
        if let Some(ref overrides) = self.request.overrides {
            for (key, value) in overrides {
                self.validate_override(key, value)?;
            }
        }

        Ok(())
    }

    /// Validates a single override parameter.
    fn validate_override(&self, key: &str, value: &Value) -> Result<(), SdkError> {
        match key {
            "temperature" => {
                if let Some(t) = value.as_f64() {
                    if !(0.0..=2.0).contains(&t) {
                        return Err(SdkError::InvalidOverride {
                            key: key.to_string(),
                            reason: "temperature must be between 0.0 and 2.0".to_string(),
                        });
                    }
                } else {
                    return Err(SdkError::InvalidOverride {
                        key: key.to_string(),
                        reason: "temperature must be a number".to_string(),
                    });
                }
            }
            "max_tokens" => {
                if let Some(t) = value.as_i64() {
                    if t <= 0 {
                        return Err(SdkError::InvalidOverride {
                            key: key.to_string(),
                            reason: "max_tokens must be positive".to_string(),
                        });
                    }
                } else {
                    return Err(SdkError::InvalidOverride {
                        key: key.to_string(),
                        reason: "max_tokens must be an integer".to_string(),
                    });
                }
            }
            "top_p" => {
                if let Some(t) = value.as_f64() {
                    if !(0.0..=1.0).contains(&t) {
                        return Err(SdkError::InvalidOverride {
                            key: key.to_string(),
                            reason: "top_p must be between 0.0 and 1.0".to_string(),
                        });
                    }
                } else {
                    return Err(SdkError::InvalidOverride {
                        key: key.to_string(),
                        reason: "top_p must be a number".to_string(),
                    });
                }
            }
            "frequency_penalty" | "presence_penalty" => {
                if let Some(p) = value.as_f64() {
                    if !(-2.0..=2.0).contains(&p) {
                        return Err(SdkError::InvalidOverride {
                            key: key.to_string(),
                            reason: format!("{} must be between -2.0 and 2.0", key),
                        });
                    }
                } else {
                    return Err(SdkError::InvalidOverride {
                        key: key.to_string(),
                        reason: format!("{} must be a number", key),
                    });
                }
            }
            "n" => {
                if let Some(n) = value.as_i64() {
                    if n <= 0 || n > 128 {
                        return Err(SdkError::InvalidOverride {
                            key: key.to_string(),
                            reason: "n must be between 1 and 128".to_string(),
                        });
                    }
                } else {
                    return Err(SdkError::InvalidOverride {
                        key: key.to_string(),
                        reason: "n must be an integer".to_string(),
                    });
                }
            }
            _ => {} // Allow unknown overrides
        }
        Ok(())
    }

    /// Finalises the builder returning a [`FluentRequest`].
    pub fn build(self) -> anyhow::Result<FluentRequest> {
        // Use the validation method
        self.validate()?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_missing_engine() {
        let builder = FluentRequestBuilder::default().request("test prompt");
        let result = builder.validate();
        assert!(matches!(result, Err(SdkError::MissingField(field)) if field == "engine"));
    }

    #[test]
    fn test_validate_missing_request() {
        let builder = FluentRequestBuilder::default().engine(EngineTemplate::OpenAIChatCompletions);
        let result = builder.validate();
        assert!(matches!(result, Err(SdkError::MissingField(field)) if field == "request"));
    }

    #[test]
    fn test_validate_empty_request() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("");
        let result = builder.validate();
        assert!(matches!(result, Err(SdkError::MissingField(field)) if field == "request"));
    }

    #[test]
    fn test_validate_invalid_temperature_too_high() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!(3.0));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "temperature" && reason.contains("between 0.0 and 2.0")
        ));
    }

    #[test]
    fn test_validate_invalid_temperature_too_low() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!(-0.1));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "temperature" && reason.contains("between 0.0 and 2.0")
        ));
    }

    #[test]
    fn test_validate_invalid_temperature_not_number() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!("not a number"));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "temperature" && reason.contains("must be a number")
        ));
    }

    #[test]
    fn test_validate_valid_temperature() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!(0.7));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_max_tokens_negative() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("max_tokens", json!(-100));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "max_tokens" && reason.contains("must be positive")
        ));
    }

    #[test]
    fn test_validate_invalid_max_tokens_zero() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("max_tokens", json!(0));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "max_tokens" && reason.contains("must be positive")
        ));
    }

    #[test]
    fn test_validate_invalid_max_tokens_not_integer() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("max_tokens", json!(100.5));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "max_tokens" && reason.contains("must be an integer")
        ));
    }

    #[test]
    fn test_validate_valid_max_tokens() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("max_tokens", json!(1000));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_top_p_too_high() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("top_p", json!(1.5));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "top_p" && reason.contains("between 0.0 and 1.0")
        ));
    }

    #[test]
    fn test_validate_invalid_top_p_negative() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("top_p", json!(-0.1));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "top_p" && reason.contains("between 0.0 and 1.0")
        ));
    }

    #[test]
    fn test_validate_valid_top_p() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("top_p", json!(0.9));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_frequency_penalty_too_high() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("frequency_penalty", json!(2.5));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "frequency_penalty" && reason.contains("between -2.0 and 2.0")
        ));
    }

    #[test]
    fn test_validate_invalid_frequency_penalty_too_low() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("frequency_penalty", json!(-2.5));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "frequency_penalty" && reason.contains("between -2.0 and 2.0")
        ));
    }

    #[test]
    fn test_validate_valid_frequency_penalty() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("frequency_penalty", json!(0.5));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_presence_penalty() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("presence_penalty", json!(3.0));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "presence_penalty" && reason.contains("between -2.0 and 2.0")
        ));
    }

    #[test]
    fn test_validate_valid_presence_penalty() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("presence_penalty", json!(-0.5));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_n_too_high() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("n", json!(129));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "n" && reason.contains("between 1 and 128")
        ));
    }

    #[test]
    fn test_validate_invalid_n_zero() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("n", json!(0));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, reason })
            if key == "n" && reason.contains("between 1 and 128")
        ));
    }

    #[test]
    fn test_validate_valid_n() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("n", json!(5));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_unknown_override_allowed() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("custom_param", json!("custom_value"));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_multiple_overrides() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!(0.8))
            .override_param("max_tokens", json!(500))
            .override_param("top_p", json!(0.95));
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validate_multiple_overrides_with_invalid() {
        let builder = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test")
            .override_param("temperature", json!(0.8))
            .override_param("max_tokens", json!(-100))
            .override_param("top_p", json!(0.95));
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidOverride { key, .. })
            if key == "max_tokens"
        ));
    }

    #[test]
    fn test_build_with_valid_params() {
        let result = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test prompt")
            .override_param("temperature", json!(0.7))
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_with_invalid_params() {
        let result = FluentRequestBuilder::default()
            .engine(EngineTemplate::OpenAIChatCompletions)
            .request("test prompt")
            .override_param("temperature", json!(5.0))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_sdk_error_display() {
        let error = SdkError::MissingField("test_field".to_string());
        assert_eq!(error.to_string(), "Missing required field: test_field");

        let error = SdkError::InvalidOverride {
            key: "temperature".to_string(),
            reason: "out of range".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Invalid override: temperature - out of range"
        );
    }
}
