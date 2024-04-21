use log::{debug, error};
use std::env;
use reqwest::{Client, Error};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::{json, Value};
use std::time::Duration;
use crate::config::{FlowConfig, replace_with_env_var};


use serde::{Deserialize, Serialize};
use serde_json::Result;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncReadExt;
use crate::client;

#[derive(Serialize, Deserialize, Debug)]
struct FluentCliOutput {
    pub(crate) text: String,
    pub(crate) question: String,
    #[serde(rename = "chatId")]
    pub(crate) chat_id: String,
    #[serde(rename = "chatMessageId")]
    chat_message_id: String,
    #[serde(rename = "sessionId")]
    pub(crate) session_id: String,
    #[serde(rename = "memoryType")]
    memory_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Question {
    question: String,
}


#[derive(Serialize, Deserialize)]
struct RequestPayload {
    question: String,
    overrideConfig: std::collections::HashMap<String, String>,
    uploads: Option<Vec<Upload>>,
}

#[derive(Serialize, Deserialize)]
struct Upload {
    data: String,
    r#type: String,
    name: String,
    mime: String,
}


pub fn handle_response(response_body: &str) -> Result<()> {
    let parsed_output: FluentCliOutput = serde_json::from_str(response_body)?;
    let question_parsed: Result<Question> = serde_json::from_str(&parsed_output.question);
    let question_text = match question_parsed {
        Ok(q) => q.question,
        Err(_) => parsed_output.question.clone(), // If parsing fails, use the original string
    };

    let code_blocks = extract_code_blocks(&parsed_output.text);
    // Print parsed data or further process it as needed
    println!("\n\n");
    println!("\tText:\n\t{}\n", parsed_output.text);
    println!("\tQuestion:\n\t{}", question_text);
    println!("\tChat ID: {}", parsed_output.chat_id);
    println!("\tSession ID: {}", parsed_output.session_id);
    println!("\tMemory Type: {:?}", parsed_output.memory_type);
    println!("\tCode blocks: {:?}", code_blocks);

    Ok(())
}



fn extract_code_blocks(markdown_content: &str) -> Vec<String> {
    let re = Regex::new(r"```[\w]*\n([\s\S]*?)\n```").unwrap();
    re.captures_iter(markdown_content)
        .map(|cap| {
            cap[1].trim().to_string()  // Trim to remove leading/trailing whitespace
        })
        .collect()
}


pub fn parse_fluent_cli_output(json_data: &str) -> Result<FluentCliOutput> {
    let output: FluentCliOutput = serde_json::from_str(json_data)?;
    Ok(output)
}


// Change the signature to accept a simple string for `question`

pub async fn send_request(flow: &FlowConfig,  payload: &Value) -> reqwest::Result<String> {
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


    let url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow.request_path, flow.chat_id);
    debug!("URL: {}", url);
    debug!("Body: {}", payload);
    debug!("Headers: {:?}", bearer_token);
    // Send the request and await the response
    let response = client.post(&url)
        .header("Authorization", format!("Bearer {}", bearer_token))
        .json(payload)
        .send()
        .await?;

    debug!("Request URL: {}", url);
    debug!("Request bearer token: {}", bearer_token);
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

    // Start building the payload with the question
    let mut payload = json!({
        "question": full_question,  // Use the potentially modified question
    });

    // Add the context to the payload if it exists
    if let Some(ctx) = context {
        payload.as_object_mut().unwrap().insert("context".to_string(), serde_json::Value::String(ctx.to_string()));
    }

    payload

}



use tokio::fs::File as TokioFile; // Alias to avoid confusion with std::fs::File
use tokio::io::{AsyncReadExt as TokioAsyncReadExt, Result as IoResult};
use base64::encode;
use std::collections::HashMap;
use std::path::Path;
use pulldown_cmark::{Event, Parser, Tag};
use regex::Regex;


pub(crate) async fn prepare_payload(flow: &FlowConfig, question: &str, file_path: Option<&str>, actual_final_context: Option<String>) -> IoResult<Value> {
    let mut override_config = flow.override_config.clone();
    // Ensure override_config is up-to-date with environment variables
    replace_with_env_var(&mut override_config);
    debug!("Override config after update: {:?}", override_config);

    debug!("File path: {:?}", file_path);
    debug!("Actual final context: {:?}", actual_final_context);

    let full_question = if let Some(ctx) = actual_final_context {
        format!("{} {}", question, ctx)  // Concatenate question and context
    } else {
        question.to_string()  // Use question as is if no context
    };
    // Assuming replace_with_env_var function exists and mutates override_config appropriately

    let mut body = json!({
        "question": full_question,
        "overrideConfig": override_config,
    });


    if let Some(path) = file_path {

        // Properly handle the file open result
        let mut file = TokioFile::open(path).await?;  // Correctly use .await and propagate errors with ?

        let mut buffer = Vec::new();
        // Use read_to_end on the file object directly
        TokioAsyncReadExt::read_to_end(&mut file, &mut buffer).await?;  // Correct usage with error propagation

        let encoded_image = encode(&buffer);  // Encode the buffer content to Base64
        let uploads = json!([{
            "data": format!("data:image/png;base64,{}", encoded_image),
            "type": "file",
            "name": path.rsplit('/').next().unwrap_or("unknown"),
            "mime": "image/png"
        }]);

        body.as_object_mut().unwrap().insert("uploads".to_string(), uploads);
    }

    Ok(body)
}

