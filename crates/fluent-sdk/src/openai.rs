use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{EngineTemplate, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentOpenAIChatRequest {}

/// Request data used for OpenAI chat completions.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentOpenAIChatRequest {
    pub prompt: String,
    pub openai_key: String,
    pub model: Option<String>,
    pub response_format: Option<Value>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub top_p: Option<f64>,
    pub n: Option<i8>,
    pub stop: Option<Vec<String>>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
}
impl FluentOpenAIChatRequest {
    /// Creates a new [`FluentOpenAIChatRequestBuilder`].
    pub fn builder() -> FluentOpenAIChatRequestBuilder {
        FluentOpenAIChatRequestBuilder::default()
    }
}
impl From<FluentOpenAIChatRequest> for FluentRequest {
    fn from(request: FluentOpenAIChatRequest) -> Self {
        let mut overrides = vec![];
        if let Some(response_format) = request.response_format {
            overrides.push(("response_format".to_string(), response_format));
        }
        if let Some(temperature) = request.temperature {
            overrides.push(("temperature".to_string(), json!(temperature)));
        }
        if let Some(max_tokens) = request.max_tokens {
            overrides.push(("max_tokens".to_string(), json!(max_tokens)));
        }
        if let Some(top_p) = request.top_p {
            overrides.push(("top_p".to_string(), json!(top_p)));
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            overrides.push(("frequency_penalty".to_string(), json!(frequency_penalty)));
        }
        if let Some(presence_penalty) = request.presence_penalty {
            overrides.push(("presence_penalty".to_string(), json!(presence_penalty)));
        }
        if let Some(model_name) = request.model {
            overrides.push(("modelName".to_string(), json!(model_name)));
        }
        if let Some(n) = request.n {
            overrides.push(("n".to_string(), json!(n)));
        }
        if let Some(stop) = request.stop {
            overrides.push(("stop".to_string(), json!(stop)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineTemplate::OpenAIChatCompletions),
            credentials: Some(vec![KeyValue::new("OPENAI_API_KEY", &request.openai_key)]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

/// Builder for [`FluentOpenAIChatRequest`].
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentOpenAIChatRequestBuilder {
    request: FluentOpenAIChatRequest,
}
impl Default for FluentOpenAIChatRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentOpenAIChatRequest {
                prompt: String::new(),
                openai_key: String::new(),
                response_format: None,
                temperature: Some(0.7),
                max_tokens: None,
                top_p: Some(1.0),
                frequency_penalty: None,
                presence_penalty: None,
                model: Some("gpt-3.5-turbo".to_string()),
                n: None,
                stop: None,
            },
        }
    }
}

impl FluentOpenAIChatRequestBuilder {
    /// Sets the user prompt.
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    /// Sets the OpenAI API key.
    pub fn openai_key(mut self, openai_key: String) -> Self {
        self.request.openai_key = openai_key;
        self
    }
    /// Overrides the `response_format` parameter.
    pub fn response_format(mut self, response_format: Value) -> Self {
        self.request.response_format = Some(response_format);
        self
    }
    /// Overrides the `temperature` parameter.
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.request.temperature = Some(temperature);
        self
    }
    /// Overrides the `max_tokens` parameter.
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.request.max_tokens = Some(max_tokens);
        self
    }
    /// Overrides the `top_p` parameter.
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.request.top_p = Some(top_p);
        self
    }
    /// Overrides the `frequency_penalty` parameter.
    pub fn frequency_penalty(mut self, frequency_penalty: f64) -> Self {
        self.request.frequency_penalty = Some(frequency_penalty);
        self
    }
    /// Overrides the `presence_penalty` parameter.
    pub fn presence_penalty(mut self, presence_penalty: f64) -> Self {
        self.request.presence_penalty = Some(presence_penalty);
        self
    }
    /// Overrides the model to use.
    pub fn model(mut self, model: String) -> Self {
        self.request.model = Some(model);
        self
    }
    /// Overrides the `n` parameter.
    pub fn n(mut self, n: i8) -> Self {
        self.request.n = Some(n);
        self
    }
    /// Overrides the `stop` parameter.
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.request.stop = Some(stop);
        self
    }
    /// Builds the request returning an error if required fields are missing.
    pub fn build(self) -> anyhow::Result<FluentOpenAIChatRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.openai_key.is_empty() {
            return Err(anyhow!("OpenAI key is required"));
        }
        Ok(self.request)
    }
}
