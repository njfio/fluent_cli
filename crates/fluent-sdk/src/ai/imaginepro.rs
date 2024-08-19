use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{EngineName, FluentRequest, KeyValue};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentImagineProRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub reference: Option<String>,
    pub webhook_override: Option<String>,
}
impl From<FluentImagineProRequest> for FluentRequest {
    fn from(request: FluentImagineProRequest) -> Self {
        let mut overrides = vec![];
        if let Some(reference) = request.reference {
            overrides.push(("ref".to_string(), json!(reference)));
        }
        if let Some(webhook_override) = request.webhook_override {
            overrides.push(("webhookOverride".to_string(), json!(webhook_override)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::ImaginePro),
            credentials: Some(vec![KeyValue::new(
                "IMAGINEPRO_API_KEY",
                &request.bearer_token,
            )]),
            overrides: Some(overrides.into_iter().collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentImagineProRequestBuilder {
    request: FluentImagineProRequest,
}
impl Default for FluentImagineProRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentImagineProRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                reference: None,
                webhook_override: None,
            },
        }
    }
}

impl FluentImagineProRequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.bearer_token = bearer_token;
        self
    }
    pub fn reference(mut self, reference: String) -> Self {
        self.request.reference = Some(reference);
        self
    }
    pub fn webhook_override(mut self, webhook_override: String) -> Self {
        self.request.webhook_override = Some(webhook_override);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentImagineProRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("OpenAI key is required"));
        }
        Ok(self.request)
    }
}
