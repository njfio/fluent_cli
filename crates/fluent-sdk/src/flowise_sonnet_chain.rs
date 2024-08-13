use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentFlowiseSonnetChainRequest {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentFlowiseSonnetChainRequest {
    pub prompt: String,
    pub anthropic_api_key: String,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub session_id: Option<String>,
}

impl From<FluentFlowiseSonnetChainRequest> for FluentRequest {
    fn from(request: FluentFlowiseSonnetChainRequest) -> Self {
        let mut overrides = vec![];
        if let Some(temperature) = request.temperature {
            overrides.push(("temperature".to_string(), json!(temperature)));
        }
        if let Some(max_tokens) = request.max_tokens {
            overrides.push(("maxTokensToSample".to_string(), json!(max_tokens)));
        }
        if let Some(model_name) = request.model {
            overrides.push(("modelName".to_string(), json!(model_name)));
        }
        if let Some(session_id) = request.session_id {
            overrides.push(("sessionID".to_string(), json!(session_id)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::FlowiseSonnet35Chain),
            credentials: Some(vec![KeyValue::new(
                "ANTHROPIC_API_KEY",
                &request.anthropic_api_key,
            )]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentFlowiseSonnetChainRequestBuilder {
    request: FluentFlowiseSonnetChainRequest,
}
impl Default for FluentFlowiseSonnetChainRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentFlowiseSonnetChainRequest {
                prompt: String::new(),
                anthropic_api_key: String::new(),
                temperature: None,
                max_tokens: None,
                model: None,
                session_id: None,
            },
        }
    }
}

impl FluentFlowiseSonnetChainRequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.anthropic_api_key = bearer_token;
        self
    }
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.request.temperature = Some(temperature);
        self
    }
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.request.max_tokens = Some(max_tokens);
        self
    }
    pub fn model(mut self, model: String) -> Self {
        self.request.model = Some(model);
        self
    }
    pub fn session_id(mut self, session_id: String) -> Self {
        self.request.session_id = Some(session_id);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentFlowiseSonnetChainRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.anthropic_api_key.is_empty() {
            return Err(anyhow!("OpenAI key is required"));
        }
        Ok(self.request)
    }
}
