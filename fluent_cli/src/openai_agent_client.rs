use reqwest::{Client, multipart};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::env;
use clap::ArgMatches;

use log::{debug, error};
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use serde::ser::StdError;
use tokio::fs::File as TokioFile; // Alias to avoid confusion with std::fs::File
use tokio::io::AsyncReadExt as TokioAsyncReadExt;

use tokio::time::{Instant, sleep};
use uuid::Uuid;

use crate::client::{handle_openai_assistant_response, resolve_env_var};
use crate::config::{FlowConfig, replace_with_env_var};
use crate::neo4j_client::{capture_llm_interaction, LlmProvider, Neo4jClient, Neo4jClientError};

#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<u64>,
    pub model: Option<String>,
    pub choices: Option<Vec<Choice>>,
    pub error: Option<OpenAIError>,  // Add this to capture errors
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OpenAIError {  // Define the error struct
    pub code: Option<String>,
    pub message: String,
    pub param: Option<String>,
    pub type_: String,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Option<Message>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Content {
    Text { r#type: String, text: String },
    Image { r#type: String, image_url: ImageUrl },
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageUrl {
    url: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
}



#[derive(Debug, Deserialize)]
pub struct OpenAIAssistantResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}


#[derive(Debug, Serialize)]
pub struct OpenAIAssistantRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
}


pub async fn create_thread(api_key: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .post("https://api.openai.com/v1/threads")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({}))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let thread_id = response["id"].as_str().ok_or("Failed to create thread")?.to_string();
    Ok(thread_id)
}

pub async fn add_message_to_thread(api_key: &str, thread_id: &str, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let _response = client
        .post(&format!("https://api.openai.com/v1/threads/{}/messages", thread_id))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&message)
        .send()
        .await?;
    Ok(())
}

pub async fn create_run(api_key: &str, thread_id: &str, assistant_id: &str, instructions: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .post(&format!("https://api.openai.com/v1/threads/{}/runs", thread_id))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({
            "assistant_id": assistant_id,
            "instructions": instructions
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    let run_id = response["id"].as_str().ok_or("Failed to create run")?.to_string();
    Ok(run_id)
}

pub async fn get_run_status(api_key: &str, thread_id: &str, run_id: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .get(&format!("https://api.openai.com/v1/threads/{}/runs/{}", thread_id, run_id))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(response)
}

pub async fn get_thread_messages(api_key: &str, thread_id: &str) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .get(&format!("https://api.openai.com/v1/threads/{}/messages", thread_id))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .send()
        .await?
        .json::<Value>()
        .await?;
    Ok(response)
}

pub async fn submit_tool_outputs(api_key: &str, thread_id: &str, run_id: &str, tool_outputs: Value) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let response = client
        .post(&format!("https://api.openai.com/v1/threads/{}/runs/{}/submit_tool_outputs", thread_id, run_id))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("OpenAI-Beta", "assistants=v2")
        .json(&json!({
            "tool_outputs": tool_outputs
        }))
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(response)
}

pub async fn handle_openai_assistant(prompt: &str, flow: &FlowConfig, matches: &ArgMatches) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);
    let model = flow.override_config["modelName"].as_str().unwrap_or("gpt-4o");
    let api_key = env::var("FLUENT_OPENAI_API_KEY_01").expect("FLUENT_OPENAI_API_KEY_01 not set");
    debug!("Using OpenAI API key: {}", api_key);

    let assistant_id = flow.override_config["assistant_id"].as_str().expect("assistant_id not set");
    debug!("Assistant ID: {}", assistant_id);
    let instructions = flow.override_config["instructions"].as_str().expect("instructions not set");
    debug!("Instructions: {}", instructions);
    let thread_id = create_thread(&api_key).await?;
    debug!("Thread ID: {}", thread_id);
    let user_message = Message { role: "user".to_string(), content: prompt.to_string() };
    debug!("User message: {:?}", user_message);
    add_message_to_thread(&api_key, &thread_id, &user_message).await?;

    let run_id = create_run(&api_key, &thread_id, assistant_id, instructions).await?;
    debug!("Run ID: {}", run_id);

    // Set a timeout for the loop
    let start_time = Instant::now();
    let timeout = flow.timeout_ms.expect("timeout_ms not set");
    let timeout_duration = Duration::from_secs(timeout / 1000);
    debug!("Timeout duration: {:?}", timeout_duration);
    let model = flow.override_config["modelName"].as_str().unwrap_or("gpt-4o");

    // Store processed tool_call_ids to avoid duplication
    let mut processed_tool_calls = std::collections::HashSet::new();

    // Poll for the run status
    loop {
        let run_status_response = get_run_status(&api_key, &thread_id, &run_id).await?;
        let status = run_status_response["status"].as_str().ok_or("Failed to get run status")?;
        debug!("Run status: {}", status);

        if status == "completed" || status == "failed" || status == "cancelled" {
            break;
        }

        if status == "requires_action" {
            debug!("Required action: {:?}", run_status_response["required_action"]);
            if let Some(required_action) = run_status_response["required_action"].as_object() {
                if let Some(tool_calls) = required_action.get("submit_tool_outputs").and_then(|v| v.get("tool_calls")).and_then(|v| v.as_array()) {
                    let mut tool_outputs = Vec::new();
                    for tool_call in tool_calls {
                        if let Some(tool_call_id) = tool_call.get("id").and_then(|v| v.as_str()) {
                            if processed_tool_calls.contains(tool_call_id) {
                                debug!("Skipping already processed tool call: {}", tool_call_id);
                                continue;
                            }
                            let output = get_tool_output(tool_call).await?;
                            tool_outputs.push(json!({
                                "tool_call_id": tool_call_id,
                                "output": output
                            }));
                            processed_tool_calls.insert(tool_call_id.to_string());
                        } else {
                            error!("Missing 'id' in tool_call: {:?}", tool_call);
                        }
                    }

                    if !tool_outputs.is_empty() {
                        debug!("Submitting tool outputs: {:?}", tool_outputs);
                        let submission_response = submit_tool_outputs(&api_key, &thread_id, &run_id, json!(tool_outputs)).await?;
                        debug!("Tool output submission response: {:?}", submission_response);
                    }
                } else {
                    error!("Missing 'tool_calls' in required_action: {:?}", required_action);
                }
            } else {
                error!("Missing 'required_action' in run_status_response: {:?}", run_status_response);
            }
        }

        if start_time.elapsed() > timeout_duration {
            return Err("Timeout while waiting for run to complete".into());
        }

        sleep(Duration::from_secs(5)).await;
    }

    // Retrieve and return the final assistant message
    let thread_messages = get_thread_messages(&api_key, &thread_id).await?;
    debug!("Thread messages: {:?}", thread_messages);

    if let Some(last_message) = thread_messages["data"].as_array().and_then(|msgs| msgs.iter().rev().find(|msg| msg["role"] == "assistant")) {
        if let Some(content_array) = last_message["content"].as_array() {
            let mut full_response = String::new();
            for content_item in content_array {
                if let Some(text) = content_item.get("text").and_then(|txt| txt.get("value")).and_then(|v| v.as_str()) {
                    full_response.push_str(text);
                }
            }
            debug!("Connection to neo4j");


            return Ok(full_response);
        }
    }

    Err("Failed to get assistant response".into())
}


pub async fn get_tool_output(_tool_call: &Value) -> Result<String, Box<dyn Error + Send + Sync>> {
    // Implement the logic to handle tool calls and generate the appropriate output
    // For demonstration purposes, we return a dummy output
    Ok("tool_output_example".to_string())
}




pub async fn send_openai_request(
    messages: Vec<Message>,
    api_key: &str,
    url: &str,
    model: &str,
    temperature: f32,
) -> Result<OpenAIResponse, Box<dyn StdError + Send + Sync>> {
    let client = reqwest::Client::new();
    let request_body = OpenAIRequest {
        model: model.to_string(),
        messages,
        temperature,
    };
    debug!("Request body: {:?}", request_body);
    let response = client
        .post(url)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        debug!("Error response body: {}", error_text);
        return Err(format!("Request failed with status: {}", status).into());
    }

    let openai_response: OpenAIResponse = response.json().await?;
    debug!("OpenAI response: {:?}", openai_response);

    if let Some(error) = openai_response.error {
        return Err(format!("OpenAI API error: {}", error.message).into());
    }

    Ok(openai_response)
}

pub async fn get_openai_embedding(
    text: &str,
    api_key: &str,
    model: &str,
) -> Result<Vec<f32>, Neo4jClientError> {
    let client = reqwest::Client::new();
    let url = "https://api.openai.com/v1/embeddings";

    let request_body = serde_json::json!({
        "input": text,
        "model": model
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| Neo4jClientError::OpenAIError(Box::new(e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.map_err(|e| Neo4jClientError::OpenAIError(Box::new(e)))?;
        debug!("Error response body: {}", error_text);
        return Err(Neo4jClientError::OpenAIError(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Request failed with status: {}", status),
        ))));
    }

    let embedding_response: serde_json::Value = response.json().await.map_err(|e| Neo4jClientError::OpenAIError(Box::new(e)))?;
    let embedding = embedding_response["data"][0]["embedding"]
        .as_array()
        .ok_or_else(|| Neo4jClientError::OpenAIError(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Embedding not found in response",
        ))))?
        .iter()
        .map(|v| v.as_f64().unwrap() as f32)
        .collect();

    Ok(embedding)
}

async fn encode_image(image_path: &Path) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut file = TokioFile::open(image_path).await?;
    let mut buffer = Vec::new();
    TokioAsyncReadExt::read_to_end(&mut file, &mut buffer).await?;
    Ok(STANDARD.encode(&buffer))
}


pub async fn handle_openai_agent(
    prompt: &str,
    flow: &FlowConfig,
    matches: &ArgMatches,
) -> Result<String, Box<dyn StdError + Send + Sync>> {
    let flow_clone = Arc::new(flow.clone());
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);
    let model = flow.override_config["modelName"].as_str().unwrap_or("gpt-4o");
    // Resolve the API key from the environment variable if it starts with "AMBER_"
    let api_key = resolve_env_var(&flow.bearer_token)?;

    debug!("Using OpenAI API key: {}", api_key);

    let url = format!("{}://{}:{}/{}", flow.protocol, flow.hostname, flow.port, flow.request_path);

    let model = flow.override_config["modelName"].as_str().unwrap_or("gpt-4o");
    let temperature = flow.override_config["temperature"].as_f64().unwrap_or(0.7) as f32;
    let max_iterations = flow.override_config["max_iterations"].as_u64().unwrap_or(10) as usize;

    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: flow.override_config["systemMessage"]
                .as_str()
                .unwrap_or("You are a helpful assistant.")
                .to_string(),
        },
        Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];

    if let Some(file_path) = matches.get_one::<String>("upload-image-path") {
        let path = Path::new(file_path);
        let encoded_image = encode_image(path).await?;

        // Add image data as a base64 string
        messages.push(Message {
            role: "user".to_string(),
            content: format!("data:image/jpeg;base64,{}", encoded_image),
        });
    }

    debug!("API key: {}", api_key);
    debug!("Protocol: {}", flow.protocol);
    debug!("Hostname: {}", flow.hostname);
    debug!("Port: {}", flow.port);
    debug!("Request path: {}", flow.request_path);
    debug!("System message: {}", flow.override_config["systemMessage"].as_str().unwrap_or("You are a helpful assistant."));
    debug!("Prompt: {}", prompt);
    debug!("Model: {}", model);
    debug!("Temperature: {}", temperature);
    debug!("Max iterations: {}", max_iterations);
    debug!("Messages: {:?}", messages);
    debug!("Model: {}", model);
    debug!("Temperature: {}", temperature);
    debug!("Max iterations: {}", max_iterations);
    debug!("URL: {}", url);

    // Initialize the Neo4j client

    // Capture the session information
    let session_id = env::var("FLUENT_SESSION_ID_01").expect("FLUENT_SESSION_ID_01 not set");
    let chat_id = flow.session_id;
    let chat_message_id = Uuid::new_v4().to_string(); // Replace with actual chat message ID if available

    let mut full_response = String::new();
    for _ in 0..max_iterations {
        let openai_response = send_openai_request(messages.clone(), &api_key, &url, model, temperature).await?;
        debug!("OpenAI response: {:?}", openai_response);
        for choice in openai_response.choices.unwrap_or_default() {
            debug!("Choice: {:?}", &choice);
            if let Some(message) = choice.message {
                full_response.push_str(&message.content);
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: message.content.clone(),
                });

            }
            debug!("Full response: {}", full_response);
            debug!("Messages: {:?}", messages);

            debug!("Choice finish reason: {:?}", choice.finish_reason);
            if choice.finish_reason.as_deref() != Some("length") {
                // If finish_reason is not "length", we have the complete response
                debug!("connection to neo4j");
                let neo4j_client = Arc::new(Neo4jClient::initialize().await?);

                capture_llm_interaction(
                    neo4j_client.clone(),
                    &flow_clone,
                    &messages.last().unwrap().content,
                    full_response.as_str(),
                    &model,
                    &openai_response.id.unwrap_or_default(),
                    LlmProvider::OpenAI
                ).await?;
                debug!("captured_llm_interaction");
                return Ok(full_response);
            }
        }
    }

    debug!("Full response: {}", full_response);
    Ok(full_response)
}





