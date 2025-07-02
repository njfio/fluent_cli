use fluent_core::config::EngineConfig;
use fluent_core::types::Request;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Utility for building request payloads consistently across engines
pub struct PayloadBuilder;

impl PayloadBuilder {
    /// Build a standard chat completion payload
    pub fn build_chat_payload(request: &Request, model: Option<&str>) -> Value {
        let mut payload = json!({
            "messages": [
                {
                    "role": "user",
                    "content": request.payload
                }
            ]
        });

        if let Some(model_name) = model {
            payload["model"] = json!(model_name);
        }

        payload
    }

    /// Build payload with additional parameters from config
    pub fn build_chat_payload_with_config(
        request: &Request, 
        config: &EngineConfig,
        model: Option<&str>
    ) -> Value {
        let mut payload = Self::build_chat_payload(request, model);

        // Add common parameters from config
        if let Some(temperature) = config.parameters.get("temperature") {
            payload["temperature"] = temperature.clone();
        }

        if let Some(max_tokens) = config.parameters.get("max_tokens") {
            payload["max_tokens"] = max_tokens.clone();
        }

        if let Some(top_p) = config.parameters.get("top_p") {
            payload["top_p"] = top_p.clone();
        }

        if let Some(frequency_penalty) = config.parameters.get("frequency_penalty") {
            payload["frequency_penalty"] = frequency_penalty.clone();
        }

        if let Some(presence_penalty) = config.parameters.get("presence_penalty") {
            payload["presence_penalty"] = presence_penalty.clone();
        }

        if let Some(stream) = config.parameters.get("stream") {
            payload["stream"] = stream.clone();
        }

        payload
    }

    /// Build image generation payload
    pub fn build_image_payload(prompt: &str, config: &EngineConfig) -> Value {
        let mut payload = json!({
            "prompt": prompt
        });

        // Add image-specific parameters
        if let Some(size) = config.parameters.get("size") {
            payload["size"] = size.clone();
        }

        if let Some(quality) = config.parameters.get("quality") {
            payload["quality"] = quality.clone();
        }

        if let Some(style) = config.parameters.get("style") {
            payload["style"] = style.clone();
        }

        if let Some(n) = config.parameters.get("n") {
            payload["n"] = n.clone();
        }

        if let Some(response_format) = config.parameters.get("response_format") {
            payload["response_format"] = response_format.clone();
        }

        payload
    }

    /// Build vision payload with image
    pub fn build_vision_payload(
        text: &str, 
        image_data: &str, 
        image_format: &str
    ) -> Value {
        json!({
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": text
                        },
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/{};base64,{}", image_format, image_data)
                            }
                        }
                    ]
                }
            ]
        })
    }

    /// Build webhook payload
    pub fn build_webhook_payload(
        request: &Request,
        config: &EngineConfig,
        file_content: Option<&str>
    ) -> Value {
        let mut payload = json!({
            "input": request.payload,
            "chat_id": config.parameters.get("chat_id").and_then(|v| v.as_str()).unwrap_or(""),
            "sessionId": config.parameters.get("sessionId").and_then(|v| v.as_str()).unwrap_or(""),
        });

        if let Some(content) = file_content {
            payload["file_content"] = json!(content);
        }

        // Add overrideConfig parameters
        if let Some(override_config) = config.parameters.get("overrideConfig") {
            if let Some(obj) = override_config.as_object() {
                for (key, value) in obj {
                    payload[key] = value.clone();
                }
            }
        }

        payload
    }

    /// Merge additional parameters into payload
    pub fn merge_parameters(
        mut payload: Value, 
        parameters: &HashMap<String, Value>
    ) -> Value {
        if let Some(obj) = payload.as_object_mut() {
            for (key, value) in parameters {
                obj.insert(key.clone(), value.clone());
            }
        }
        payload
    }

    /// Extract model name from config with fallback
    pub fn get_model_name(config: &EngineConfig, default: &str) -> String {
        config.parameters
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::config::{EngineConfig, ConnectionConfig};

    fn create_test_request() -> Request {
        Request {
            flowname: "test".to_string(),
            payload: "Hello, world!".to_string(),
        }
    }

    fn create_test_config() -> EngineConfig {
        let mut parameters = HashMap::new();
        parameters.insert("temperature".to_string(), json!(0.7));
        parameters.insert("max_tokens".to_string(), json!(100));
        parameters.insert("model".to_string(), json!("gpt-3.5-turbo"));

        EngineConfig {
            name: "test".to_string(),
            engine: "test".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.example.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters,
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[test]
    fn test_build_chat_payload() {
        let request = create_test_request();
        let payload = PayloadBuilder::build_chat_payload(&request, Some("gpt-4"));
        
        assert_eq!(payload["model"], "gpt-4");
        assert_eq!(payload["messages"][0]["role"], "user");
        assert_eq!(payload["messages"][0]["content"], "Hello, world!");
    }

    #[test]
    fn test_build_chat_payload_with_config() {
        let request = create_test_request();
        let config = create_test_config();
        let payload = PayloadBuilder::build_chat_payload_with_config(&request, &config, None);
        
        assert_eq!(payload["temperature"], 0.7);
        assert_eq!(payload["max_tokens"], 100);
        assert_eq!(payload["messages"][0]["content"], "Hello, world!");
    }

    #[test]
    fn test_get_model_name() {
        let config = create_test_config();
        let model = PayloadBuilder::get_model_name(&config, "default-model");
        assert_eq!(model, "gpt-3.5-turbo");

        let mut empty_config = config.clone();
        empty_config.parameters.clear();
        let default_model = PayloadBuilder::get_model_name(&empty_config, "default-model");
        assert_eq!(default_model, "default-model");
    }
}
