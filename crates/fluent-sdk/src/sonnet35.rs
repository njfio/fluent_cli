use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentSonnet35Request {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentSonnet35Request {
    pub prompt: String,
    pub bearer_token: String,
    pub system: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
}
impl From<FluentSonnet35Request> for FluentRequest {
    fn from(request: FluentSonnet35Request) -> Self {
        let mut overrides = vec![];
        if let Some(temperature) = request.temperature {
            overrides.push(("temperature".to_string(), json!(temperature)));
        }
        if let Some(max_tokens) = request.max_tokens {
            overrides.push(("max_tokens".to_string(), json!(max_tokens)));
        }
        if let Some(model_name) = request.model {
            overrides.push(("modelName".to_string(), json!(model_name)));
        }
        if let Some(system) = request.system {
            overrides.push(("system".to_string(), json!(system)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::Sonnet35),
            credentials: Some(vec![KeyValue::new(
                "ANTHROPIC_API_KEY",
                &request.bearer_token,
            )]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentSonnet35RequestBuilder {
    request: FluentSonnet35Request,
}
impl Default for FluentSonnet35RequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentSonnet35Request {
                prompt: String::new(),
                bearer_token: String::new(),
                temperature: None,
                max_tokens: None,
                model: None,
                system: None,
            },
        }
    }
}

impl FluentSonnet35RequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.bearer_token = bearer_token;
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
    pub fn system(mut self, system: String) -> Self {
        self.request.system = Some(system);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentSonnet35Request> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("Bearer Token is required"));
        }
        Ok(self.request)
    }
}
