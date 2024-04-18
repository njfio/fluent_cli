use log::{debug, error};
use std::env;
use reqwest::{Client, Error};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use std::time::Duration;
use crate::config::FlowConfig;

// Change the signature to accept a simple string for `question`

pub async fn send_request(flow: &FlowConfig, question: &str) -> Result<String, Error> {
    let client_builder = Client::builder();

    // Set timeout if specified in config
    let client = if let Some(timeout_ms) = flow.timeout_ms {
        client_builder.timeout(Duration::from_millis(timeout_ms)).build()?
    } else {
        client_builder.build()?
    };

    // Construct the request URL
    let url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow.request_path, flow.chat_id);
    debug!("Request URL: {}", url);

    // Fetch the bearer token from environment variables
    let bearer_token = env::var("bearer_token").unwrap_or_else(|_| {
        error!("Bearer token not found in environment variables.");
        flow.bearer_token.clone() // Fallback to the value from config if not found
    });

    // Prepare the request body
    let body = json!({
        "question": question,
        "overrideConfig": &flow.override_config
    });

    debug!("Sending request to: {}", url);
    debug!("Request body: {:?}", body);
    debug!("Bearer token used: {}", bearer_token);

    // Send the request and await the response
    let response = client.post(&url)
        .header(AUTHORIZATION, format!("Bearer {}", bearer_token))
        .json(&body)
        .send()
        .await?;

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
