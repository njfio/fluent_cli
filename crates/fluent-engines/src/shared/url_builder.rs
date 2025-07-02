use fluent_core::config::EngineConfig;

/// Utility for building URLs consistently across engines
pub struct UrlBuilder;

impl UrlBuilder {
    /// Build a complete URL from engine config and path
    pub fn build_url(config: &EngineConfig, path: &str) -> String {
        let base_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };

        format!(
            "{}://{}:{}{}",
            config.connection.protocol,
            config.connection.hostname,
            config.connection.port,
            base_path
        )
    }

    /// Build URL using the request_path from config
    pub fn build_default_url(config: &EngineConfig) -> String {
        Self::build_url(config, &config.connection.request_path)
    }

    /// Build URL for a specific endpoint
    pub fn build_endpoint_url(config: &EngineConfig, endpoint: &str) -> String {
        Self::build_url(config, endpoint)
    }

    /// Validate URL format
    pub fn validate_url(url: &str) -> bool {
        reqwest::Url::parse(url).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::config::{EngineConfig, ConnectionConfig};
    use std::collections::HashMap;

    fn create_test_config() -> EngineConfig {
        EngineConfig {
            name: "test".to_string(),
            engine: "test".to_string(),
            connection: ConnectionConfig {
                protocol: "https".to_string(),
                hostname: "api.example.com".to_string(),
                port: 443,
                request_path: "/v1/chat/completions".to_string(),
            },
            parameters: HashMap::new(),
            session_id: None,
            neo4j: None,
            spinner: None,
        }
    }

    #[test]
    fn test_build_url() {
        let config = create_test_config();
        let url = UrlBuilder::build_url(&config, "/test/endpoint");
        assert_eq!(url, "https://api.example.com:443/test/endpoint");
    }

    #[test]
    fn test_build_url_without_leading_slash() {
        let config = create_test_config();
        let url = UrlBuilder::build_url(&config, "test/endpoint");
        assert_eq!(url, "https://api.example.com:443/test/endpoint");
    }

    #[test]
    fn test_build_default_url() {
        let config = create_test_config();
        let url = UrlBuilder::build_default_url(&config);
        assert_eq!(url, "https://api.example.com:443/v1/chat/completions");
    }

    #[test]
    fn test_validate_url() {
        assert!(UrlBuilder::validate_url("https://api.example.com/test"));
        assert!(UrlBuilder::validate_url("http://localhost:8080/api"));
        assert!(!UrlBuilder::validate_url("invalid-url"));
        assert!(!UrlBuilder::validate_url(""));
    }
}
