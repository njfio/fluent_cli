use std::collections::HashMap;
use log::{debug};
use std::env;
use reqwest::{Client};

use serde_json::{json, Value};

use crate::config::{FlowConfig, replace_with_env_var};

use serde::{Deserialize, Serialize};
use serde_json::Result;
use tokio::fs::File;

use tokio::io::AsyncReadExt;

#[derive(Debug, Deserialize)]
pub struct FluentCliOutput {
    pub text: String,
    pub question: Option<String>,
    #[serde(rename = "chatId")]
    pub chat_id: Option<String>,
    #[serde(rename = "chatMessageId")]
    pub chat_message_id: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(rename = "memoryType")]
    pub memory_type: Option<String>,
    #[serde(rename = "sourceDocuments")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_documents: Option<Vec<Option<SourceDocument>>>,
    #[serde(rename = "agentReasoning")]
    pub agent_reasoning: Option<Vec<AgentReasoning>>,
}

#[derive(Debug, Deserialize)]
pub struct AgentReasoning {
    #[serde(rename = "agentName")]
    pub agent_name: String,
    pub messages: Vec<String>,
    pub next: Option<String>,
    pub instructions: Option<String>,
    #[serde(rename = "usedTools")]
    pub used_tools: Option<Vec<Option<String>>>,
    #[serde(rename = "sourceDocuments")]
    pub source_documents: Option<Vec<Option<SourceDocument>>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceDocument {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_content: Option<String>,
    pub metadata: Option<Metadata>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>, // Make repository optional
    pub branch: String,
    pub loc: Location,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Location {
    pub lines: Lines,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lines {
    pub from: i32,
    pub to: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct Question {
    question: String,
}

#[derive(Serialize, Deserialize)]
struct RequestPayload {
    question: String,
    #[serde(rename="overrideConfig")]
    override_config: std::collections::HashMap<String, String>,
    uploads: Option<Vec<Upload>>,
}

#[derive(Serialize, Deserialize)]
struct Upload {
    data: String,
    r#type: String,
    name: String,
    mime: String,
}

#[derive(Debug)]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct ResponseOutput {
    response_text: Option<String>,
    question: Option<String>,
    chat_id: Option<String>,
    session_id: Option<String>,
    memory_type: Option<String>,
    code_blocks: Option<Vec<String>>,
    pretty_text: Option<String>,
    source: Option<String>,
}

use std::collections::{ HashSet};

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowOutput {
    pub(crate) session_id: String,
    pub(crate) outputs: Vec<LangFlowOutputDetail>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowOutputDetail {
    pub(crate) inputs: LangFlowInput,
    pub(crate) outputs: Vec<LangFlowResultDetail>,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowInput {
    pub(crate) input_value: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowResultDetail {
    pub(crate) results: LangFlowResult,
    pub(crate) artifacts: Option<LangFlowArtifacts>,
    pub(crate) messages: Vec<LangFlowMessage>,
    #[serde(rename = "component_display_name")]
    pub(crate) component_display_name: String,
    #[serde(rename = "component_id")]
    pub(crate) component_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowResult {
    pub(crate) result: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowArtifacts {
    pub(crate) message: String,
    pub(crate) sender: String,
    #[serde(rename = "sender_name")]
    pub(crate) sender_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LangFlowMessage {
    pub(crate) message: String,
    pub(crate) sender: String,
    #[serde(rename = "sender_name")]
    pub(crate) sender_name: String,
    #[serde(rename = "component_id")]
    pub(crate) component_id: String,
}

pub async fn handle_langflow_response(response_body: &str, matches: &clap::ArgMatches) -> Result<()> {
    debug!("LangFlow response body: {}", response_body);
    let result = serde_json::from_str::<LangFlowOutput>(response_body);
    debug!("Parsed LangFlow result: {:?}", result);

    match result {
        Ok(lang_flow_output) => {
            // Use a HashSet to collect unique messages
            let mut response_texts: HashSet<String> = HashSet::new();

            for output in lang_flow_output.outputs {
                for detail in output.outputs {
                    if !detail.results.result.is_empty() {
                        response_texts.insert(detail.results.result.clone());
                    }
                    if let Some(artifacts) = &detail.artifacts {
                        if !artifacts.message.is_empty() {
                            response_texts.insert(artifacts.message.clone());
                        }
                    }
                    for message in &detail.messages {
                        if !message.message.is_empty() {
                            response_texts.insert(message.message.clone());
                        }
                    }
                }
            }

            let response_text = response_texts.into_iter().collect::<Vec<String>>().join("\n");

            if !matches.get_one::<bool>("markdown-output").map_or(false, |&v| v) &&
                matches.get_one::<bool>("parse-code-output").map_or(true, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                let code_blocks = extract_code_blocks(&response_text);
                for block in code_blocks {
                    println!("{}", block);
                }
                return Ok(());
            }

            if let Some(directory) = matches.get_one::<String>("download-media").map(|s| s.as_str()) {
                let urls = extract_urls(&response_text); // Adjust the URL extraction as needed
                download_media(urls, directory).await;
            }

            if matches.get_one::<bool>("markdown-output").map_or(true, |&v| v) &&
                !matches.get_one::<bool>("parse-code-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                pretty_format_markdown(&response_text);
                return Ok(());
            }



            if !matches.get_one::<bool>("markdown-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("parse-code-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                println!("{}", response_text);  // Default output
                return Ok(());
            }
        },
        Err(e) => {
            eprintln!("Error parsing LangFlow response: {:?}", e);
            if let Some(directory) = matches.get_one::<String>("download-media").map(|s| s.as_str()) {
                let urls = extract_urls(response_body); // Fallback to raw response
                download_media(urls, directory).await;
            }
            println!("{}", response_body); // Print raw response if there is a parsing error
        }
    }

    Ok(())
}


pub async fn handle_response(response_body: &str, matches: &clap::ArgMatches) -> Result<()> {
    debug!("Response body: {}", response_body);

    // Parse the response body as a generic JSON value
    let result: Result<Value> = serde_json::from_str(response_body);
    debug!("Result: {:?}", result);

    match result {
        Ok(parsed_output) => {
            // If parsing is successful, use the parsed data
            debug!("Parsed Output: {:?}", parsed_output);

            // Extract text field if available
            if let Some(text) = parsed_output.get("text").and_then(Value::as_str) {
                debug!("parsed_output text: {}", text);
            }

            // Extract agent reasoning details if present
            if let Some(agent_reasoning) = parsed_output.get("agentReasoning").and_then(Value::as_array) {
                eprintln!("\nAgent Reasoning Details:");
                for agent in agent_reasoning {
                    if let Some(agent_name) = agent.get("agentName").and_then(Value::as_str) {
                        eprintln!("Agent Name: {}", agent_name);
                    }
                    if let Some(messages) = agent.get("messages").and_then(Value::as_array) {
                        eprintln!("Messages:");
                        for message in messages {
                            if let Some(msg) = message.as_str() {
                                eprintln!("- {}", msg);
                            }
                        }
                    }
                    if let Some(next) = agent.get("next").and_then(Value::as_str) {
                        eprintln!("Next Step: {}", next);
                    }
                    if let Some(instructions) = agent.get("instructions").and_then(Value::as_str) {
                        eprintln!("Instructions: {}", instructions);
                    }
                    if let Some(used_tools) = agent.get("usedTools").and_then(Value::as_array) {
                        eprintln!("Used Tools:");
                        for tool in used_tools {
                            if let Some(tool_name) = tool.as_str() {
                                eprintln!("- {}", tool_name);
                            }
                        }
                    }
                    eprintln!("\n---\n");
                }
            }

            if !matches.get_one::<bool>("markdown-output").map_or(false, |&v| v) &&
                matches.get_one::<bool>("parse-code-output").map_or(true, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                debug!("parse code");
                if let Some(text) = parsed_output.get("text").and_then(Value::as_str) {
                    let code_blocks = extract_code_blocks(text);
                    for block in code_blocks {
                        println!("{}", block);
                    }
                }
                return Ok(());
            }

            if let Some(directory) = matches.get_one::<String>("download-media").map(|s| s.as_str()) {
                let urls = extract_urls(response_body); // Assume extract_urls can handle any text
                download_media(urls, directory).await;
            }

            if matches.get_one::<bool>("markdown-output").map_or(true, |&v| v) &&
                !matches.get_one::<bool>("parse-code-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                if let Some(text) = parsed_output.get("text").and_then(Value::as_str) {
                    pretty_format_markdown(text);
                }
                if let Some(documents) = parsed_output.get("sourceDocuments").and_then(Value::as_array) {
                    pretty_format_markdown("\n---\n");
                    pretty_format_markdown("\n\n# Source Documents\n");
                    pretty_format_markdown("\n---\n");
                    for doc_option in documents {
                        if let Some(doc) = doc_option.as_object() {
                            let markdown_link = format!(
                                "[View Source]({}/blob/{}/{}#L{}-L{})",
                                doc.get("metadata")
                                    .and_then(|meta| meta.get("repository"))
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                doc.get("metadata")
                                    .and_then(|meta| meta.get("branch"))
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                doc.get("metadata")
                                    .and_then(|meta| meta.get("source"))
                                    .and_then(Value::as_str)
                                    .unwrap_or(""),
                                doc.get("metadata")
                                    .and_then(|meta| meta.get("loc"))
                                    .and_then(|loc| loc.get("lines"))
                                    .and_then(|lines| lines.get("from"))
                                    .and_then(Value::as_i64)
                                    .unwrap_or(0),
                                doc.get("metadata")
                                    .and_then(|meta| meta.get("loc"))
                                    .and_then(|loc| loc.get("lines"))
                                    .and_then(|lines| lines.get("to"))
                                    .and_then(Value::as_i64)
                                    .unwrap_or(0)
                            );
                            pretty_format_markdown(&markdown_link);
                            if let Some(content) = doc.get("page_content").and_then(Value::as_str) {
                                pretty_format_markdown(&format!("**Page Content:**\n{}", content));
                            } else {
                                pretty_format_markdown("**Page Content:**\nNo content available");
                            }
                        }
                    }
                    pretty_format_markdown("---\n");
                }
                return Ok(());
            }


            if !matches.get_one::<bool>("markdown-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("parse-code-output").map_or(false, |&v| v) &&
                matches.get_one::<bool>("full-output").map_or(true, |&v| v) {
                debug!("full output");
                println!("{}", response_body);
                return Ok(());
            }

            if !matches.get_one::<bool>("markdown-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("parse-code-output").map_or(false, |&v| v) &&
                !matches.get_one::<bool>("full-output").map_or(false, |&v| v) {
                debug!("default");
                if let Some(text) = parsed_output.get("text").and_then(Value::as_str) {
                    println!("{}", text);
                }
                return Ok(());
            }
        },
        Err(e) => {
            // If there's an error parsing the JSON, print the error and the raw response body
            eprintln!("Failed to parse JSON, this might be normal if it's a webhook request: {}", e);
            if let Some(cause) = e.source() {
                eprintln!("Cause: {:?}", cause);
            }
            if let Some(directory) = matches.get_one::<String>("download-media").map(|s| s.as_str()) {
                let urls = extract_urls(response_body);
                // Assume extract_urls can handle any text
                debug!("Extracted URLs: {:?}", urls);
                download_media(urls, directory).await;
            }
            debug!("Download Response body: {}", response_body);
            println!("{}", response_body);
        }
    };

    Ok(())
}

fn extract_urls(text: &str) -> Vec<String> {
    let url_regex = Regex::new(r"https?://[^\s]+").unwrap();
    url_regex.find_iter(text)
        .map(|mat| mat.as_str().to_string())
        .collect()
}

pub fn print_full_width_bar(string: &str) -> String {
    let width = terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| w as usize).unwrap_or(80);
    string.repeat(width).dark_yellow().to_string()
}

use termimad::*;

fn pretty_format_markdown(markdown_content: &str) {
    let mut skin = MadSkin::default(); // Assuming `termimad` is used
    skin.bold.set_fg(crossterm::style::Color::Yellow);
    skin.italic.set_fg(crossterm::style::Color::Blue);
    skin.headers[0].set_fg(crossterm::style::Color::Yellow);
    skin.headers[1].set_fg(crossterm::style::Color::Green);
    skin.headers[2].set_fg(crossterm::style::Color::Blue);
    skin.inline_code.set_fg(crossterm::style::Color::White);
    skin.code_block.set_fg(crossterm::style::Color::White);

    skin.paragraph.left_margin = 4;
    skin.paragraph.right_margin = 4;

    skin.print_text(markdown_content);
}

fn extract_code_blocks(markdown_content: &str) -> Vec<String> {
    let re = Regex::new(r"```[\w]*\n([\s\S]*?)\n```").unwrap();
    re.captures_iter(markdown_content)
        .map(|cap| {
            cap[1].trim().to_string()  // Trim to remove leading/trailing whitespace
        })
        .collect()
}

use tokio::io::AsyncWriteExt;
use chrono::Local;

async fn download_media(urls: Vec<String>, directory: &str) {
    let client = reqwest::Client::new();

    for url in urls {
        // Trim any trailing unwanted characters such as parentheses
        let clean_url = url.trim_end_matches(')');
        debug!("Cleaned URL: {}", clean_url);

        match client.get(clean_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(content) => {
                            let path = Path::new(clean_url);
                            let filename = if let Some(name) = path.file_name() {
                                format!("{}-{}.{}", name.to_string_lossy(), Local::now().format("%Y%m%d%H%M%S"), path.extension().unwrap_or_default().to_string_lossy())
                            } else {
                                format!("download-{}.dat", Local::now().format("%Y%m%d%H%M%S"))
                            };
                            let filepath = Path::new(directory).join(filename);
                            debug!("Downloading: {}\nto: {}\n", clean_url, filepath.display());
                            match File::create(&filepath).await {
                                Ok(mut file) => {
                                    if let Err(e) = file.write_all(&content).await {
                                        eprintln!("Failed to write to file: {}", e);
                                    }
                                },
                                Err(e) => eprintln!("Failed to create file: {}", e),
                            }
                        },
                        Err(e) => eprintln!("Failed to read bytes from response: {}", e),
                    }
                } else {
                    eprintln!("Failed to download {}: {}", clean_url, response.status());
                }
            },
            Err(e) => eprintln!("Failed to send request: {}", e),
        }
    }
}

// Change the signature to accept a simple string for `question`
pub async fn send_request(flow: &FlowConfig, payload: &Value) -> reqwest::Result<String> {
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

use std::error::Error as StdError;  // Import the StdError trait for `source` method

fn to_serde_json_error<E: std::fmt::Display>(err: E) -> serde_json::Error {
    serde_json::Error::custom(err.to_string())
}

pub async fn upsert_with_json(api_url: &str, flow: &FlowConfig, payload: serde_json::Value) -> Result<()> {
    let bearer_token = if flow.bearer_token.starts_with("AMBER_") {
        env::var(&flow.bearer_token[6..]).unwrap_or_else(|_| flow.bearer_token.clone())
    } else {
        flow.bearer_token.clone()
    };

    debug!("Bearer token: {}", bearer_token);

    let client = reqwest::Client::new();
    debug!("Sending to URL: {}", api_url);
    debug!("Payload: {:?}", payload);
    eprintln!("Upserting.....");
    debug!("Upserting with JSON: {:?}", payload);
    let response = client.post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", bearer_token))
        .json(&payload)
        .send()
        .await
        .map_err(to_serde_json_error)?;

    if response.status().is_success() {
        let response_json: serde_json::Value = response.json().await.map_err(to_serde_json_error)?;
        debug!("Success response: {:#?}", response_json);
        eprintln!("Success: {:#?}", response_json);
    } else {
        let error_message = format!("Failed to upsert data: Status code: {}", response.status());
        return Err(serde_json::Error::custom(error_message));
    }

    Ok(())
}

pub async fn upload_files(api_url: &str, file_paths: Vec<&str>) -> Result<()> {
    let client = Client::new();
    let mut form = Form::new();
    let file_paths_clone = file_paths.clone();

    for file_path in file_paths {
        let path = Path::new(file_path);
        debug!("File path: {}", path.display());
        let mime_type = mime_guess::from_path(path).first_or_octet_stream().essence_str().to_string(); // Convert to String here
        debug!("MIME type: {}", mime_type);
        let mut file = match File::open(path).await {
            Ok(f) => f,
            Err(e) => return Err(to_serde_json_error(e)),
        };
        debug!("File opened: {}", file_path);
        let mut buffer = Vec::new();
        if let Err(e) = file.read_to_end(&mut buffer).await {
            return Err(to_serde_json_error(e));
        }

        let part = Part::bytes(buffer)
            .file_name(path.file_name().unwrap().to_str().unwrap().to_owned())
            .mime_str(&mime_type).map_err(to_serde_json_error)?; // Use a reference to the owned String

        form = form.part("files", part);
    }
    debug!("Form: {:?}", form);
    debug!("API URL: {}", api_url);
    debug!("File paths: {:?}", file_paths_clone);
    let response = client.post(api_url)
        .multipart(form)
        .send()
        .await
        .map_err(to_serde_json_error)?;

    debug!("Response: {:?}", response);
    if !response.status().is_success() {
        return Err(to_serde_json_error(format!("Failed to upload files: Status code: {}", response.status())));
    }

    let response_json: serde_json::Value = response.json().await.map_err(to_serde_json_error)?;
    println!("{:#?}", response_json);

    Ok(())
}

fn parse_key_value_pair(pair: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = pair.splitn(2, '=').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None
    }
}

use tokio::fs::File as TokioFile; // Alias to avoid confusion with std::fs::File
use tokio::io::{AsyncReadExt as TokioAsyncReadExt, Result as IoResult};

use std::path::Path;
use clap::ArgMatches;

use regex::Regex;
use reqwest::multipart::{Form, Part};
use serde::de::Error;

use termimad::{MadSkin};
use termimad::crossterm::style::Stylize;

use base64::{engine::general_purpose::STANDARD, Engine};
pub async fn prepare_payload(
    flow: &FlowConfig,
    question: &str,
    file_path: Option<&str>,
    actual_final_context: Option<String>,
    cli_args: &ArgMatches,
    _file_contents: &str,
) -> IoResult<Value> {
    let mut override_config = flow.override_config.clone();
    let mut tweaks_config = flow.tweaks.clone();
    debug!("Override config before update: {:?}", override_config);
    debug!("Tweaks config before update: {:?}", tweaks_config);
    replace_with_env_var(&mut tweaks_config);
    replace_with_env_var(&mut override_config); // Update config with env variables
    debug!("Override config after update: {:?}", override_config);
    debug!("Tweaks config after update: {:?}", tweaks_config);

    let full_question = actual_final_context.as_ref().map_or_else(
        || question.to_string(),
        |ctx| format!("\n{}\n{}\n", question, ctx)
    );

    debug!("Engine: {}", flow.engine);
    let mut body = match flow.engine.as_str() {
        "flowise" => {
            let system_prompt_inline = cli_args.get_one::<String>("system-prompt-override-inline").map(|s| s.as_str());
            let system_prompt_file = cli_args.get_one::<String>("system-prompt-override-file").map(|s| s.as_str());
            // Load override value from file if specified
            let system_message_override = if let Some(file_path) = system_prompt_file {
                let mut file = File::open(file_path).await?; // Corrected async file opening
                let mut contents = String::new();
                file.read_to_string(&mut contents).await?; // Read file content asynchronously
                Some(contents)
            } else {
                system_prompt_inline.map(|s| s.to_string())
            };

            // Update the configuration with the override if it exists
            // Update the configuration based on what's present in the overrideConfig
            let mut flow_clone = flow.clone();
            if let Some(override_value) = system_message_override {
                if let Some(obj) = flow_clone.override_config.as_object_mut() {
                    if obj.contains_key("systemMessage") {
                        obj.insert("systemMessage".to_string(), serde_json::Value::String(override_value.to_string()));
                    }
                    if obj.contains_key("systemMessagePrompt") {
                        obj.insert("systemMessagePrompt".to_string(), serde_json::Value::String(override_value.to_string()));
                    }
                }
            }

            debug!("Flowise Engine");
            serde_json::json!({
                "question": full_question,
                "overrideConfig": override_config,
            })
        },
        "langflow" => {
            debug!("Langflow Engine");

            // Update the tweaks config with the full question at the specified input value key
            if let Some(input_value_key) = flow.input_value_key.as_deref() {
                if let Some(tweak) = tweaks_config.get_mut(input_value_key) {
                    if let Some(tweak_obj) = tweak.as_object_mut() {
                        tweak_obj.insert("input_value".to_string(), serde_json::Value::String(full_question.clone()));
                    }
                }
            }

            // Process overrides for tweaks
            if let Some(overrides) = cli_args.get_many::<String>("override") {
                let overrides: HashMap<String, String> = overrides
                    .map(|s| parse_key_value_pair(s).unwrap())
                    .collect();

                for (key, value) in overrides {
                    // Split the key into parts
                    let key_parts: Vec<&str> = key.split('.').collect();
                    if key_parts.len() == 2 {
                        if let Some(tweak) = tweaks_config.get_mut(key_parts[0]) {
                            if let Some(tweak_obj) = tweak.as_object_mut() {
                                // Only override if the value is not an empty string
                                if !value.trim().is_empty() {
                                    tweak_obj.insert(key_parts[1].to_string(), serde_json::Value::String(value.clone()));
                                }
                            }
                        }
                    }
                }
            }

            debug!("Tweaks config after update: {:?}", tweaks_config);
            serde_json::json!({
                "input_value": "message",
                "input_type": flow.input_type,
                "output_type": flow.output_type,
                "tweaks": tweaks_config,
            })
        },
        "webhook" => {
            debug!("Webhook Engine");
            serde_json::json!({
                "question": full_question,
                "context": actual_final_context,
                "file_contents": _file_contents,
                "overrideConfig": override_config,
            })
        },
        _ => {
            debug!("Unknown engine: {}", flow.engine);
            serde_json::json!({
                "question": full_question,
                "overrideConfig": override_config,
            })
        }
    };

    debug!("Body: {:?}", body);
    if cli_args.contains_id("upload-image-path") && file_path.is_some() {
        let path = file_path.unwrap();
        let mut file = TokioFile::open(path).await?;
        let mut buffer = Vec::new();
        TokioAsyncReadExt::read_to_end(&mut file, &mut buffer).await?;

        // Encoding using the STANDARD engine
        let encoded_image = STANDARD.encode(&buffer);  // Correct use of the encode method with the STANDARD engine

        let uploads = json!([{
            "data": format!("data:image/png;base64,{}", encoded_image),
            "type": "file",
            "name": Path::new(path).file_name().unwrap_or_default().to_string_lossy(),
            "mime": "image/png"
        }]);
        body.as_object_mut().unwrap().insert("uploads".to_string(), uploads);
    }

    Ok(body)
}



