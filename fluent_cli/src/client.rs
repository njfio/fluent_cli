use log::{debug, error};
use std::env;
use reqwest::{Client};
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
use serde_yaml::to_string as to_yaml;  // Add serde_yaml to your Cargo.toml if not already included


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

#[derive(Debug)]
struct ResponseOutput {
    response_text: String,
    question: String,
    chat_id: String,
    session_id: String,
    memory_type: Option<String>,
    code_blocks: Option<Vec<String>>,  // Only populated if `--parse-code-output` is present
    pretty_text: Option<String>,       // Only populated if `--parse-code-output` is not present
}

use serde_json::Error as SerdeError;

pub async fn handle_response(response_body: &str, matches: &clap::ArgMatches) -> Result<()> {
    // Parse the response body, handle error properly here instead of unwrapping
    debug!("Response body: {}", response_body);
    let result = serde_json::from_str::<FluentCliOutput>(response_body);


    let response_text = match result {
        Ok(parsed_output) => {
            // If parsing is successful, use the parsed data
            debug!("{:?}", parsed_output);
            if let Some(directory) = matches.value_of("download-media") {
                let urls = extract_urls(response_body); // Assume extract_urls can handle any text
                download_media(urls, directory).await;
            }
            if matches.is_present("markdown-output") {
                let pretty_text = pretty_format_markdown(&parsed_output.text); // Ensure text is obtained correctly
                eprintln!("{:?}", pretty_text);
            } else if matches.is_present("parse-code-output") {
                let code_blocks = extract_code_blocks(&parsed_output.text);
                for block in code_blocks {
                    println!("{}", block);
                }
            } else if matches.is_present("full-output") {
                println!("{}", response_body);  // Output the text used, whether parsed or raw
            } else {
                println!("{}", parsed_output.text);  // Output the text used, whether parsed or raw, but only if the --markdown-output flag is not set").text;
            }
        },
        Err(e) => {
            // If there's an error parsing the JSON, print the error and the raw response body
            eprintln!("Raw Webhook Output");
            if let Some(directory) = matches.value_of("download-media") {
                let urls = extract_urls(response_body); // Assume extract_urls can handle any text
                download_media(urls, directory).await;
            }
            println!("{}", response_body);
            response_body.to_string();
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

fn pretty_format_markdown(markdown_content: &str) {
    let skin = MadSkin::default(); // Assuming `termimad` is used
    let formatted = skin.print_text(markdown_content); // Render to a string
    formatted
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


use reqwest;

use tokio::io::AsyncWriteExt;

use chrono::Local;


use anyhow::{Context};

// Correct definition of the function returning a Result with a boxed dynamic error



async fn download_media(urls: Vec<String>, directory: &str) {
    let client = reqwest::Client::new();

    for url in urls {
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.bytes().await {
                        Ok(content) => {
                            let path = Path::new(&url);
                            let filename = if let Some(name) = path.file_name() {
                                format!("{}-{}.{}", name.to_string_lossy(), Local::now().format("%Y%m%d%H%M%S"), path.extension().unwrap_or_default().to_string_lossy())
                            } else {
                                format!("download-{}.dat", Local::now().format("%Y%m%d%H%M%S"))
                            };
                            let filepath = Path::new(directory).join(filename);

                            match File::create(filepath).await {
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
                    eprintln!("Failed to download {}: {}", url, response.status());
                }
            },
            Err(e) => eprintln!("Failed to send request: {}", e),
        }
    }
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
use std::io::ErrorKind;
use std::path::Path;
use pulldown_cmark::{Event, Parser, Tag};
use regex::Regex;
use serde::de::Error;
use termimad::{FmtText, MadSkin};
use termimad::minimad::once_cell::sync::Lazy;


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

