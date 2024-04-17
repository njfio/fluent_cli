use log::debug;
use reqwest::{Client, Error};
use serde_json::{json, Value};
use std::time::Duration;
use crate::config::FlowConfig;

// Change the signature to accept a simple string for `question`
pub async fn send_request(flow: &FlowConfig, question: &str) -> Result<String, Error> {
    let client_builder = reqwest::Client::builder();

    // Set timeout if specified in config
    let client = if let Some(timeout_ms) = flow.timeout_ms {
        client_builder.timeout(Duration::from_millis(timeout_ms)).build()?
    } else {
        client_builder.build()?
    };

    // Construct the request URL
    let url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow.request_path, flow.chat_id);
    debug!("Request URL: {}", url);

    // Prepare the request body using `question` as a plain string
    let body = json!({
        "question": question,  // Now using `question` as a plain string directly
        "overrideConfig": &flow.override_config
    });

    debug!("Sending request to: {}", url);
    debug!("Request body: {:?}", body);

    // Send the request and await the response
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", flow.bearer_token))
        .json(&body)
        .send()
        .await?;

    debug!("Response: {:?}", response);
    return response.text().await;
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
