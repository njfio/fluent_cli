pub mod loader;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub api: ApiServiceConfig,
    pub flows: Vec<super::api::FlowConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ApiServiceConfig {
    pub host: String,
    pub flowise_api_urls: ApiUrls,
}

#[derive(Debug, Deserialize)]
pub struct ApiUrls {
    pub message_api_get: String,
    pub message_api_delete: String,
    pub document_loader: String,
    // Add other fields as necessary
}


// Ensure that any constructors or relevant methods are also public


