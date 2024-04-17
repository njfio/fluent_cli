use log::debug;
use reqwest::{Client, Error};
use serde_json::json;
use std::time::Duration;
use crate::config::FlowConfig;

pub async fn send_request(flow: &FlowConfig, question: &str) -> Result<String, Error> {
    let client_builder = Client::builder();

    // Set timeout if specified in config
    let client = if let Some(timeout_ms) = flow.timeout_ms {
        client_builder.timeout(Duration::from_millis(timeout_ms)).build()?
    } else {
        client_builder.build()?
    };

    let url = format!("https://{}:{}{}{}", flow.hostname, flow.port, flow.request_path, flow.chat_id);
    debug!("Sending request to: {}", url);
    let body = json!({
        "question": question,
        "overrideConfig": &flow.override_config
    });
    debug!("Request body: {:?}", body);

    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", flow.bearer_token))
        .json(&body)
        .send()
        .await?;
    debug!("Response: {:?}", response);
    response.text().await
}