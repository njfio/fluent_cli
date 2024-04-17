use std::collections::HashMap;
use reqwest::{Client, Error};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FlowConfig {
    flow_name: String,
    chat_id: String,
    api_key: String,
    override_config: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ApiServiceConfig {
    pub host: String,
    pub api_urls: ApiUrls,
}

#[derive(Debug, Deserialize)]
pub struct ApiUrls {
    pub message_api_get: String,
    pub message_api_delete: String,
    pub document_loader: String,
    // Add other API endpoints as necessary
}

pub async fn make_api_call(flow_name: &str, request: &str) -> Result<String, Error> {
    let client = Client::new();
    let url = format!("https://api.flowiseai.com/flows/{}", flow_name);
    let response = client.post(url)
        .body(request.to_string())
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}
