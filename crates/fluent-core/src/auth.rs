use anyhow::{anyhow, Result};
use log::{debug, warn};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::Value;
use std::collections::HashMap;

/// Secure string that clears memory on drop
#[derive(Clone)]
pub struct SecureString {
    data: Vec<u8>,
}

impl SecureString {
    pub fn new(s: String) -> Self {
        Self {
            data: s.into_bytes(),
        }
    }

    pub fn as_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.data)
            .map_err(|e| anyhow!("Invalid UTF-8 in secure string: {}", e))
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Drop for SecureString {
    fn drop(&mut self) {
        // Securely clear the memory
        for byte in &mut self.data {
            *byte = 0;
        }
    }
}

impl std::fmt::Debug for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecureString([REDACTED] {} bytes)", self.len())
    }
}

impl std::fmt::Display for SecureString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

/// Authentication types supported by the system
#[derive(Debug, Clone)]
pub enum AuthType {
    Bearer,
    ApiKey(String), // Custom header name
    Basic { username: String, password: String },
    Custom { header: String, value: String },
}

/// Secure authentication manager
pub struct AuthManager {
    auth_type: AuthType,
    token: SecureString,
}

impl AuthManager {
    /// Creates a new authentication manager with security validation
    pub fn new(config_params: &HashMap<String, Value>, auth_type: AuthType) -> Result<Self> {
        let token = Self::extract_token_securely(config_params)?;
        Self::validate_token(&token)?;

        Ok(Self {
            auth_type,
            token: SecureString::new(token),
        })
    }

    /// Creates authentication manager for bearer token
    pub fn bearer_token(config_params: &HashMap<String, Value>) -> Result<Self> {
        Self::new(config_params, AuthType::Bearer)
    }

    /// Creates authentication manager for API key with custom header
    pub fn api_key(config_params: &HashMap<String, Value>, header_name: &str) -> Result<Self> {
        Self::new(config_params, AuthType::ApiKey(header_name.to_string()))
    }

    /// Creates authentication manager for basic auth
    pub fn basic_auth(config_params: &HashMap<String, Value>) -> Result<Self> {
        let username = config_params
            .get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Username not found in configuration"))?
            .to_string();

        let password = config_params
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Password not found in configuration"))?
            .to_string();

        Self::validate_credentials(&username, &password)?;

        // For basic auth, we store the base64 encoded credentials as the token
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            credentials.as_bytes(),
        );

        Ok(Self {
            auth_type: AuthType::Basic { username, password },
            token: SecureString::new(encoded),
        })
    }

    /// Extracts token securely from configuration
    fn extract_token_securely(config_params: &HashMap<String, Value>) -> Result<String> {
        // Try multiple possible token parameter names
        let token_keys = [
            "bearer_token",
            "api_token",
            "api_key",
            "token",
            "auth_token",
        ];

        for key in &token_keys {
            if let Some(token_value) = config_params.get(*key) {
                if let Some(token_str) = token_value.as_str() {
                    if !token_str.is_empty() {
                        debug!("Found authentication token with key: {}", key);
                        return Ok(token_str.to_string());
                    }
                }
            }
        }

        Err(anyhow!(
            "No valid authentication token found in configuration. Expected one of: {:?}",
            token_keys
        ))
    }

    /// Validates token format and security
    fn validate_token(token: &str) -> Result<()> {
        if token.is_empty() {
            return Err(anyhow!("Authentication token cannot be empty"));
        }

        if token.len() < 8 {
            return Err(anyhow!(
                "Authentication token too short (minimum 8 characters)"
            ));
        }

        if token.len() > 2048 {
            return Err(anyhow!(
                "Authentication token too long (maximum 2048 characters)"
            ));
        }

        // Check for suspicious patterns
        if token.contains(' ') || token.contains('\n') || token.contains('\r') {
            return Err(anyhow!("Authentication token contains invalid characters"));
        }

        // Warn about potentially insecure tokens
        if token.starts_with("test") || token.starts_with("demo") || token == "placeholder" {
            warn!("Authentication token appears to be a test/demo token");
        }

        Ok(())
    }

    /// Validates basic auth credentials
    fn validate_credentials(username: &str, password: &str) -> Result<()> {
        if username.is_empty() {
            return Err(anyhow!("Username cannot be empty"));
        }

        if password.is_empty() {
            return Err(anyhow!("Password cannot be empty"));
        }

        if password.len() < 8 {
            warn!("Password is shorter than recommended minimum (8 characters)");
        }

        Ok(())
    }

    /// Adds authentication headers to a request
    pub fn add_auth_headers(&self, headers: &mut HeaderMap) -> Result<()> {
        match &self.auth_type {
            AuthType::Bearer => {
                let token_str = self.token.as_str()?;
                let auth_value = format!("Bearer {}", token_str);
                let header_value = HeaderValue::from_str(&auth_value)
                    .map_err(|e| anyhow!("Invalid bearer token format: {}", e))?;
                headers.insert(AUTHORIZATION, header_value);
            }

            AuthType::ApiKey(header_name) => {
                let token_str = self.token.as_str()?;
                let header_value = HeaderValue::from_str(token_str)
                    .map_err(|e| anyhow!("Invalid API key format: {}", e))?;
                let header_name = reqwest::header::HeaderName::from_bytes(header_name.as_bytes())
                    .map_err(|e| anyhow!("Invalid header name: {}", e))?;
                headers.insert(header_name, header_value);
            }

            AuthType::Basic { .. } => {
                let token_str = self.token.as_str()?;
                let auth_value = format!("Basic {}", token_str);
                let header_value = HeaderValue::from_str(&auth_value)
                    .map_err(|e| anyhow!("Invalid basic auth format: {}", e))?;
                headers.insert(AUTHORIZATION, header_value);
            }

            AuthType::Custom { header, value } => {
                let header_value = HeaderValue::from_str(value)
                    .map_err(|e| anyhow!("Invalid custom header value: {}", e))?;
                let header_name = reqwest::header::HeaderName::from_bytes(header.as_bytes())
                    .map_err(|e| anyhow!("Invalid custom header name: {}", e))?;
                headers.insert(header_name, header_value);
            }
        }

        debug!(
            "Added authentication headers for type: {:?}",
            std::mem::discriminant(&self.auth_type)
        );
        Ok(())
    }

    /// Creates a pre-configured reqwest client with authentication
    pub fn create_authenticated_client(&self) -> Result<reqwest::Client> {
        let mut headers = HeaderMap::new();
        self.add_auth_headers(&mut headers)?;

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(client)
    }

    /// Gets the authentication type (for logging/debugging)
    pub fn auth_type_name(&self) -> &'static str {
        match self.auth_type {
            AuthType::Bearer => "Bearer Token",
            AuthType::ApiKey(_) => "API Key",
            AuthType::Basic { .. } => "Basic Auth",
            AuthType::Custom { .. } => "Custom Auth",
        }
    }

    /// Validates that the token is still valid (basic checks)
    pub fn validate_current_token(&self) -> Result<()> {
        let token_str = self.token.as_str()?;
        Self::validate_token(token_str)
    }
}

/// Engine-specific authentication helpers
pub struct EngineAuth;

impl EngineAuth {
    /// Creates authentication for OpenAI-compatible APIs
    pub fn openai(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }

    /// Creates authentication for Anthropic API
    pub fn anthropic(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::api_key(config_params, "x-api-key")
    }

    /// Creates authentication for Cohere API
    pub fn cohere(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }

    /// Creates authentication for Mistral API
    pub fn mistral(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }

    /// Creates authentication for Stability AI
    pub fn stability_ai(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }

    /// Creates authentication for Google Gemini
    pub fn google_gemini(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::api_key(config_params, "x-goog-api-key")
    }

    /// Creates authentication for Replicate
    pub fn replicate(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }

    /// Creates authentication for webhook/generic APIs
    pub fn webhook(config_params: &HashMap<String, Value>) -> Result<AuthManager> {
        AuthManager::bearer_token(config_params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_bearer_token_validation() {
        let mut config = HashMap::new();
        config.insert("bearer_token".to_string(), json!("valid_token_12345"));

        let auth = AuthManager::bearer_token(&config).unwrap();
        assert_eq!(auth.auth_type_name(), "Bearer Token");
    }

    #[test]
    fn test_invalid_token() {
        let mut config = HashMap::new();
        config.insert("bearer_token".to_string(), json!("short"));

        assert!(AuthManager::bearer_token(&config).is_err());
    }

    #[test]
    fn test_missing_token() {
        let config = HashMap::new();
        assert!(AuthManager::bearer_token(&config).is_err());
    }

    #[test]
    fn test_openai_auth_creation() {
        let mut params = HashMap::new();
        params.insert("bearer_token".to_string(), json!("test-token-123"));

        let auth = EngineAuth::openai(&params).unwrap();
        assert_eq!(auth.auth_type_name(), "Bearer Token");
    }

    #[test]
    fn test_anthropic_auth_creation() {
        let mut params = HashMap::new();
        params.insert("api_key".to_string(), json!("test-api-key"));

        let auth = EngineAuth::anthropic(&params).unwrap();
        assert_eq!(auth.auth_type_name(), "API Key");
    }

    #[test]
    fn test_google_auth_creation() {
        let mut params = HashMap::new();
        params.insert("api_key".to_string(), json!("test-api-key"));

        let auth = EngineAuth::google_gemini(&params).unwrap();
        assert_eq!(auth.auth_type_name(), "API Key");
    }

    #[tokio::test]
    async fn test_client_creation() {
        let mut params = HashMap::new();
        params.insert("bearer_token".to_string(), json!("test-token-123"));

        let auth = EngineAuth::openai(&params).unwrap();
        let client = auth.create_authenticated_client().unwrap();

        // Verify the client was created successfully
        assert!(client.get("https://httpbin.org/get").build().is_ok());
    }
}
