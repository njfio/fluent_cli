use anyhow::Result;
use fluent_core::auth::EngineAuth;
use fluent_core::config::EngineConfig;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

/// Shared HTTP client functionality for engines
pub struct EngineHttpClient {
    client: Client,
    base_url: String,
    default_headers: HashMap<String, String>,
}

impl EngineHttpClient {
    /// Create a new HTTP client for an engine
    pub fn new(config: &EngineConfig) -> Result<Self> {
        // Create authenticated client using the centralized auth system
        let auth_manager = EngineAuth::openai(&config.parameters)?;
        let client = auth_manager.create_authenticated_client()?;

        let base_url = format!(
            "{}://{}:{}",
            config.connection.protocol, config.connection.hostname, config.connection.port
        );

        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());

        Ok(Self {
            client,
            base_url,
            default_headers,
        })
    }

    /// Send a POST request with JSON payload
    pub async fn post_json(&self, path: &str, payload: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);

        let mut request = self.client.post(&url);

        // Add default headers
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        let response = request.json(payload).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP request failed with status: {} - {}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string())
            ));
        }

        let response_body = response.json::<Value>().await?;
        Ok(response_body)
    }

    /// Send a POST request with multipart form data
    pub async fn post_multipart(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Value> {
        let url = format!("{}{}", self.base_url, path);

        let response = self.client.post(&url).multipart(form).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP request failed with status: {} - {}",
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string())
            ));
        }

        let response_body = response.json::<Value>().await?;
        Ok(response_body)
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the underlying reqwest client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Add a default header
    pub fn add_default_header(&mut self, key: String, value: String) {
        self.default_headers.insert(key, value);
    }
}

/// Common HTTP utilities
pub struct HttpUtils;

impl HttpUtils {
    /// Extract error message from API response
    pub fn extract_error_message(response: &Value) -> Option<String> {
        // Try common error field names
        if let Some(error) = response.get("error") {
            if let Some(message) = error.get("message") {
                return message.as_str().map(String::from);
            }
            if let Some(error_str) = error.as_str() {
                return Some(error_str.to_string());
            }
        }

        if let Some(message) = response.get("message") {
            return message.as_str().map(String::from);
        }

        if let Some(detail) = response.get("detail") {
            return detail.as_str().map(String::from);
        }

        None
    }

    /// Check if response indicates rate limiting
    pub fn is_rate_limited(response: &Value) -> bool {
        if let Some(error) = response.get("error") {
            if let Some(code) = error.get("code") {
                if let Some(code_str) = code.as_str() {
                    return code_str.contains("rate_limit") || code_str.contains("quota");
                }
            }
            if let Some(message) = error.get("message") {
                if let Some(msg_str) = message.as_str() {
                    let msg_lower = msg_str.to_lowercase();
                    return msg_lower.contains("rate limit")
                        || msg_lower.contains("quota")
                        || msg_lower.contains("too many requests");
                }
            }
        }
        false
    }
}
