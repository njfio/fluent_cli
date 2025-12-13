use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{EngineTemplate, FluentRequest, FluentSdkRequest, KeyValue, SdkError};

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

    /// Validates the current builder state.
    pub fn validate(&self) -> Result<(), SdkError> {
        // Validate required fields
        if self.request.prompt.is_empty() {
            return Err(SdkError::MissingField("prompt".to_string()));
        }
        if self.request.openai_key.is_empty() {
            return Err(SdkError::MissingField("openai_key".to_string()));
        }

        // Validate optional parameters
        if let Some(temp) = self.request.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(SdkError::InvalidConfig {
                    field: "temperature".to_string(),
                    message: "must be between 0.0 and 2.0".to_string(),
                });
            }
        }

        if let Some(tokens) = self.request.max_tokens {
            if tokens <= 0 {
                return Err(SdkError::InvalidConfig {
                    field: "max_tokens".to_string(),
                    message: "must be positive".to_string(),
                });
            }
        }

        if let Some(top_p) = self.request.top_p {
            if !(0.0..=1.0).contains(&top_p) {
                return Err(SdkError::InvalidConfig {
                    field: "top_p".to_string(),
                    message: "must be between 0.0 and 1.0".to_string(),
                });
            }
        }

        if let Some(penalty) = self.request.frequency_penalty {
            if !(-2.0..=2.0).contains(&penalty) {
                return Err(SdkError::InvalidConfig {
                    field: "frequency_penalty".to_string(),
                    message: "must be between -2.0 and 2.0".to_string(),
                });
            }
        }

        if let Some(penalty) = self.request.presence_penalty {
            if !(-2.0..=2.0).contains(&penalty) {
                return Err(SdkError::InvalidConfig {
                    field: "presence_penalty".to_string(),
                    message: "must be between -2.0 and 2.0".to_string(),
                });
            }
        }

        if let Some(n) = self.request.n {
            if n <= 0 {
                return Err(SdkError::InvalidConfig {
                    field: "n".to_string(),
                    message: "must be positive".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Builds the request returning an error if required fields are missing.
    pub fn build(self) -> anyhow::Result<FluentOpenAIChatRequest> {
        // Use the validation method
        self.validate()?;
        Ok(self.request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_validate_missing_prompt() {
        let builder = FluentOpenAIChatRequestBuilder::default().openai_key("test-key".to_string());
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::MissingField(field)) if field == "prompt"
        ));
    }

    #[test]
    fn test_openai_validate_missing_key() {
        let builder = FluentOpenAIChatRequestBuilder::default().prompt("test prompt".to_string());
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::MissingField(field)) if field == "openai_key"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_temperature() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .temperature(3.0);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "temperature"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_max_tokens() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .max_tokens(-100);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "max_tokens"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_top_p() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .top_p(1.5);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "top_p"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_frequency_penalty() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .frequency_penalty(3.0);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "frequency_penalty"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_presence_penalty() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .presence_penalty(-3.0);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "presence_penalty"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_n() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .n(0);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "n"
        ));
    }

    #[test]
    fn test_openai_validate_invalid_n_negative() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test".to_string())
            .openai_key("key".to_string())
            .n(-1);
        let result = builder.validate();
        assert!(matches!(
            result,
            Err(SdkError::InvalidConfig { field, .. }) if field == "n"
        ));
    }

    #[test]
    fn test_openai_validate_valid_request() {
        let builder = FluentOpenAIChatRequestBuilder::default()
            .prompt("test prompt".to_string())
            .openai_key("test-key".to_string())
            .temperature(0.7)
            .max_tokens(1000)
            .top_p(0.9);
        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_openai_build_with_valid_params() {
        let result = FluentOpenAIChatRequestBuilder::default()
            .prompt("test prompt".to_string())
            .openai_key("test-key".to_string())
            .temperature(0.8)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_openai_build_with_invalid_temperature() {
        let result = FluentOpenAIChatRequestBuilder::default()
            .prompt("test prompt".to_string())
            .openai_key("test-key".to_string())
            .temperature(5.0)
            .build();
        assert!(result.is_err());
    }
}
