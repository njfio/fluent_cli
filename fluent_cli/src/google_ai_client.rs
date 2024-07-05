use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use reqwest::Client;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use clap::ArgMatches;
use log::{debug, error};
use serde::de::StdError;
use tokio::fs::File as TokioFile; // Alias to avoid confusion with std::fs::File
use tokio::io::AsyncReadExt as TokioAsyncReadExt;
use tokio::time::Instant;
use uuid::Uuid;
use crate::client::resolve_env_var;

use crate::config::{FlowConfig, replace_with_env_var};
use crate::neo4j_client::{Neo4jClient, LlmProvider, capture_llm_interaction};



#[derive(Debug, Deserialize)]
struct GoogleAIError {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
    param: Option<String>,
    code: Option<String>,
}


#[derive(Debug, Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct PartResponse {
    text: String,
}


#[derive(Debug, Serialize, Deserialize)]
struct GoogleAIRequest {
    contents: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GoogleAIResponse {
    candidates: Vec<Candidate>,
    usageMetadata: UsageMetadata,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: CandidateContent,
    finishReason: String,
    index: usize,
    safetyRatings: Vec<SafetyRating>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<CandidatePart>,
    role: String,
}

#[derive(Debug, Deserialize)]
struct CandidatePart {
    text: String,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct SafetyRating {
    category: String,
    probability: String,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    promptTokenCount: u32,
    candidatesTokenCount: u32,
    totalTokenCount: u32,
}

async fn encode_image(image_path: &Path) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut file = TokioFile::open(image_path).await?;
    let mut buffer = Vec::new();
    TokioAsyncReadExt::read_to_end(&mut file, &mut buffer).await?;
    Ok(STANDARD.encode(&buffer))
}

pub async fn send_google_ai_request(
    prompt: &str,
    api_key: &str,
    url: &str,
) -> Result<GoogleAIResponse, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let request_body = GoogleAIRequest {
        contents: vec![Content {
            parts: vec![Part { text: prompt.to_string() }],
        }],
    };
    debug!("Request body: {:?}", request_body);
    let response = client
        .post(url)
        .header("Content-Type", "application/json")

        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;
        debug!("Error response body: {}", error_text);
        return Err(format!("Request failed with status: {}", status).into());
    }

    // Log the raw response text for debugging
    let raw_response = response.text().await?;
    debug!("Raw response body: {}", raw_response);

    // Deserialize the response into GoogleAIResponse
    let google_ai_response: GoogleAIResponse = serde_json::from_str(&raw_response)?;
    debug!("Google AI response: {:?}", google_ai_response);

    Ok(google_ai_response)
}



pub async fn handle_google_gemini_agent(
    prompt: &str,
    flow: &FlowConfig,
    matches: &ArgMatches,
) -> Result<String, Box<dyn StdError + Send + Sync>> {
    let mut flow = flow.clone();
    replace_with_env_var(&mut flow.override_config);

    let api_key = resolve_env_var(&flow.bearer_token)?;

    debug!("Using Google AI API key: {}", api_key);

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        flow.override_config["modelName"].as_str().unwrap_or("gemini-1.5-pro-latest"),
        api_key
    );

    let mut prompt_message = prompt.to_string();
    let model = flow.override_config["modelName"].as_str().unwrap_or("gemini-1.5-pro-latest");
    if let Some(file_path) = matches.get_one::<String>("upload-image-path") {
        let path = Path::new(file_path);
        let encoded_image = encode_image(path).await?;

        prompt_message = format!(
            "{}\n\n![Image](data:image/jpeg;base64,{})",
            prompt_message, encoded_image
        );
    }

    debug!("Prompt message: {}", prompt_message);

    let start_time = Instant::now();
    let google_response = send_google_ai_request(&prompt_message, &api_key, &url).await?;
    let duration = start_time.elapsed();

    debug!("Google AI response: {:?}", google_response);
    if let Some(generated_text) = google_response.candidates.first().and_then(|c| c.content.parts.first().map(|p| p.text.clone())) {
        debug!("Generated text: {}", generated_text);

        // Prepare the full response JSON
        let full_response_json = serde_json::json!({
            "model": model,
            "generated_text": generated_text,
            "finish_reason": google_response.candidates.first().map(|c| c.finishReason.clone()),
            "safety_ratings": google_response.candidates.first().map(|c| &c.safetyRatings),
            "usage": {
                "prompt_tokens": google_response.usageMetadata.promptTokenCount,
                "completion_tokens": google_response.usageMetadata.candidatesTokenCount,
                "total_tokens": google_response.usageMetadata.totalTokenCount
            }
        });

        // Initialize Neo4jClient
        let neo4j_client = Arc::new(Neo4jClient::initialize().await?);

        // Capture the LLM interaction in Neo4j
        if let Err(e) = capture_llm_interaction(
            neo4j_client,
            &flow,
            &prompt_message,
            &generated_text,
            model,
            &serde_json::to_string(&full_response_json).unwrap(),
            LlmProvider::Google,
        ).await {
            error!("Failed to capture LLM interaction: {:?}", e);
        }

        Ok(generated_text)
    } else if let Some(error) = google_response.candidates.first().and_then(|c| c.safetyRatings.first()) {
        debug!("Error from Google AI API: {}", error.category);
        Err(format!("Google AI API error: {}", error.probability).into())
    } else {
        Err("No generated text or error message in the response".into())
    }
}