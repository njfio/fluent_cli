use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentGeminiProRequest {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentGeminiProRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub top_k: Option<i64>,
    pub top_p: Option<f64>,
    pub session_id: Option<String>,
}
impl From<FluentGeminiProRequest> for FluentRequest {
    fn from(request: FluentGeminiProRequest) -> Self {
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
        if let Some(top_k) = request.top_k {
            overrides.push(("top_k".to_string(), json!(top_k)));
        }
        if let Some(top_p) = request.top_p {
            overrides.push(("top_p".to_string(), json!(top_p)));
        }
        if let Some(session_id) = request.session_id {
            overrides.push(("sessionId".to_string(), json!(session_id)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::GeminiPro),
            credentials: Some(vec![KeyValue::new("GOOGLE_API_KEY", &request.bearer_token)]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentGeminiProRequestBuilder {
    request: FluentGeminiProRequest,
}
impl Default for FluentGeminiProRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentGeminiProRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                temperature: None,
                max_tokens: None,
                model: None,
                top_k: None,
                top_p: None,
                session_id: None,
            },
        }
    }
}

impl FluentGeminiProRequestBuilder {
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
    pub fn top_k(mut self, top_k: i64) -> Self {
        self.request.top_k = Some(top_k);
        self
    }
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.request.top_p = Some(top_p);
        self
    }
    pub fn session_id(mut self, session_id: String) -> Self {
        self.request.session_id = Some(session_id);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentGeminiProRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("Bearer Token is required"));
        }
        Ok(self.request)
    }
}
