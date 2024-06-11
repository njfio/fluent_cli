use reqwest::Client;
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::env;
use clap::ArgMatches;

use log::{debug, error};
use std::error::Error;
use std::time::Duration;
use serde::ser::StdError;

use tokio::time::{Instant, sleep};

use crate::client::{handle_openai_assistant_response};
use crate::config::{FlowConfig, replace_with_env_var};

#[derive(Debug, Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
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
                            let output = get_tool_output(tool_call).await?;
                            tool_outputs.push(json!({
                                "tool_call_id": tool_call_id,
                                "output": output
                            }));
                        } else {
                            error!("Missing 'id' in tool_call: {:?}", tool_call);
                        }
                    }

                    debug!("Submitting tool outputs: {:?}", tool_outputs);
                    let submission_response = submit_tool_outputs(&api_key, &thread_id, &run_id, json!(tool_outputs)).await?;
                    debug!("Tool output submission response: {:?}", submission_response);
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

        sleep(Duration::from_secs(2)).await;
    }

    // Retrieve the messages from the thread
    let thread_messages = get_thread_messages(&api_key, &thread_id).await?;
    debug!("Thread messages: {:?}", thread_messages);

    // Call handle_openai_assistant_response with the raw JSON string
    let response_body = serde_json::to_string(&thread_messages)?;
    handle_openai_assistant_response(&response_body, matches).await?;

    // Extract the assistant's response from thread messages
    if let Some(last_message) = thread_messages["data"].as_array().and_then(|msgs| msgs.iter().rev().find(|msg| msg["role"] == "assistant")) {
        if let Some(content) = last_message.get("content").and_then(|c| c.as_array()).and_then(|arr| arr.iter().find_map(|item| item.get("text").and_then(|txt| txt.get("value")).and_then(Value::as_str))) {
            return Ok(content.to_string());
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

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    Ok(response)
}




pub async fn handle_openai_agent(prompt: &str, flow: &FlowConfig, _matches: &ArgMatches) -> Result<String, Box<dyn StdError + Send + Sync>> {
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);

    let api_key = env::var("FLUENT_OPENAI_API_KEY_01").expect("FLUENT_OPENAI_API_KEY_01 not set");
    debug!("Using OpenAI API key: {}", api_key);

    let url = format!("{}://{}:{}/{}", flow.protocol, flow.hostname, flow.port, flow.request_path);

    let model = flow.override_config["modelName"]["chatOpenAICustom_0"].as_str().unwrap_or("gpt-4o");
    let temperature = flow.override_config["temperature"].as_f64().unwrap_or(0.7) as f32;
    let max_iterations = flow.override_config["max_iterations"].as_u64().unwrap_or(10) as usize;

    let mut messages = vec![
        Message { role: "system".to_string(), content: flow.override_config["systemMessage"].as_str().unwrap_or("You are a helpful assistant.").to_string() },
        Message { role: "user".to_string(), content: prompt.to_string() },
    ];
    debug!("Messages: {:?}", messages);
    debug!("Model: {}", model);
    debug!("Temperature: {}", temperature);
    debug!("Max iterations: {}", max_iterations);
    debug!("URL: {}", url);


    let mut full_response = String::new();
    for _ in 0..max_iterations {
        let openai_response = send_openai_request(messages.clone(), &api_key, &url, model, temperature).await?;
        debug!("OpenAI response: {:?}", openai_response);
        for choice in openai_response.choices {
            debug!("Choice: {:?}", &choice);
            if let Some(message) = choice.message {
                full_response.push_str(&message.content);
                messages.push(Message { role: "assistant".to_string(), content: message.content.clone() });
            }
            debug!("Full response: {}", full_response);
            debug!("Messages: {:?}", messages);

            debug!("Choice finish reason: {:?}", choice.finish_reason);
            if choice.finish_reason.as_deref() != Some("length") {
                // If finish_reason is not "length", we have the complete response
                return Ok(full_response);
            }
        }
    }

    debug!("Full response: {}", full_response);
    Ok(full_response)
}



