use log::{debug, error};
use std::env;
use reqwest::{Client, Error};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use std::time::Duration;
use crate::config::{FlowConfig, replace_with_env_var};

// Change the signature to accept a simple string for `question`
pub async fn send_request(flow: &FlowConfig, question: &str) -> Result<String, Error> {
    let client = Client::new();

    // Dynamically fetch the bearer token from environment variables if it starts with "AMBER_"
    let bearer_token = if flow.bearer_token.starts_with("AMBER_") {
        env::var(&flow.bearer_token[6..]).unwrap_or_else(|_| flow.bearer_token.clone())
    } else {
        flow.bearer_token.clone()
    };
    debug!("Bearer token: {}", bearer_token);
    // Ensure override_config is up-to-date with environment variables
    let mut override_config = flow.override_config.clone();
    debug!("Override config before update: {:?}", override_config);
    replace_with_env_var(&mut override_config);
    debug!("Override config after update: {:?}", override_config);

    // Construct the body of the request
    let body = json!({
        "question": question,
        "overrideConfig": override_config
    });

    let url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow.request_path, flow.chat_id);

    // Send the request and await the response
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .json(&body)
        .send()
        .await?;

    debug!("Request URL: {}", url);
    debug!("Request bearer token: {}", bearer_token);
    debug!("Request body: {:?}", body);
    debug!("Response: {:?}", response);

    response.text().await
}


pub(crate) fn build_request_payload(question: &str, context: Option<&str>) -> Value {
    // Construct the basic question
    let full_question = if let Some(ctx) = context {
        format!("{} {}", question, ctx)  // Concatenate question and context
    } else {
        question.to_string()  // Use question as is if no context
    };

    // Now create the payload with the potentially modified question
    let payload = json!({
        "question": full_question,  // Use the potentially modified question
    });

    debug!("Request payload: {:?}", payload);
    payload
}
