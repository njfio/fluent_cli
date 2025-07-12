//! Core traits for the Fluent CLI system
//!
//! This module defines the fundamental traits that enable the plugin
//! architecture and provide abstractions for LLM engines, file handling,
//! and other core functionality.

use crate::config::EngineConfig;
use crate::neo4j_client::Neo4jClient;
use crate::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::debug;
use pdf_extract::extract_text;
use serde_json::{json, Value};
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

/// Trait for handling file uploads and processing
///
/// Provides functionality for uploading files to LLM services
/// and processing requests that include file attachments.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::traits::FileUpload;
/// use fluent_core::types::Request;
/// use std::path::Path;
///
/// async fn upload_document<T: FileUpload>(uploader: &T, path: &Path) -> anyhow::Result<String> {
///     uploader.upload_file(path).await
/// }
/// ```
#[async_trait]
pub trait FileUpload: Send + Sync {
    /// Upload a file and return its identifier
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to upload
    ///
    /// # Returns
    ///
    /// A string identifier for the uploaded file
    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;

    /// Process a request that includes a file attachment
    ///
    /// # Arguments
    ///
    /// * `request` - The request to process
    /// * `file_path` - Path to the file to include
    ///
    /// # Returns
    ///
    /// A response from the LLM that processed the request and file
    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;
}

/// Core trait for LLM engine implementations
///
/// This trait defines the interface that all LLM engines must implement
/// to work with the Fluent CLI system. It provides methods for executing
/// requests, managing data, and handling files.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::traits::Engine;
/// use fluent_core::types::Request;
///
/// async fn chat_with_engine<T: Engine>(engine: &T, message: &str) -> anyhow::Result<String> {
///     let request = Request {
///         flowname: "chat".to_string(),
///         payload: message.to_string(),
///     };
///     let response = engine.execute(&request).await?;
///     Ok(response.content)
/// }
/// ```
#[async_trait]
pub trait Engine: Send + Sync {
    /// Execute a request against the LLM engine
    ///
    /// This is the primary method for sending requests to the LLM
    /// and receiving responses.
    ///
    /// # Arguments
    ///
    /// * `request` - The request containing the flow name and payload
    ///
    /// # Returns
    ///
    /// A response containing the generated content, usage stats, and cost
    fn execute<'a>(
        &'a self,
        request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;

    /// Upsert data into the knowledge base
    ///
    /// Stores conversation history, document content, or other data
    /// for future retrieval and context.
    ///
    /// # Arguments
    ///
    /// * `request` - The upsert request containing input, output, and metadata
    ///
    /// # Returns
    ///
    /// A response indicating what was processed and any errors
    fn upsert<'a>(
        &'a self,
        request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a>;

    /// Get the Neo4j client for graph database operations
    ///
    /// # Returns
    ///
    /// An optional reference to the Neo4j client if available
    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>>;

    /// Get the current session identifier
    ///
    /// # Returns
    ///
    /// An optional session ID string for tracking conversations
    fn get_session_id(&self) -> Option<String>;

    /// Extract structured content from a JSON value
    ///
    /// Parses LLM responses to extract structured information
    /// like sentiment, themes, and keywords.
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to extract content from
    ///
    /// # Returns
    ///
    /// Extracted content structure if parsing succeeds
    fn extract_content(&self, value: &Value) -> Option<ExtractedContent>;

    /// Upload a file to the LLM service
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to upload
    ///
    /// # Returns
    ///
    /// A string identifier for the uploaded file
    fn upload_file<'a>(
        &'a self,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;
    fn process_request_with_file<'a>(
        &'a self,
        request: &'a Request,
        file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;
}

pub trait EngineConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value>;
}

pub struct AnthropicConfigProcessor;
impl EngineConfigProcessor for AnthropicConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("AnthropicConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "messages": [
                {
                    "role": "user",
                    "content": "" // This will be filled later with the actual request
                }
            ],
            "model": config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Model name not specified or not a string"))?,
        });

        // Add other parameters, excluding 'sessionID'
        for (key, value) in &config.parameters {
            if key != "sessionID" && !["modelName", "bearer_token"].contains(&key.as_str()) {
                match key.as_str() {
                    "max_tokens" => {
                        if let Some(num) = value.as_str().and_then(|s| s.parse::<i64>().ok()) {
                            payload[key] = json!(num);
                        } else if let Some(num) = value.as_i64() {
                            payload[key] = json!(num);
                        }
                    }
                    "temperature" | "top_p" => {
                        if let Some(num) = value.as_str().and_then(|s| s.parse::<f64>().ok()) {
                            payload[key] = json!(num);
                        } else if let Some(num) = value.as_f64() {
                            payload[key] = json!(num);
                        }
                    }
                    _ => {
                        payload[key] = value.clone();
                    }
                }
            }
        }

        debug!("Anthropic Payload: {:#?}", payload);
        Ok(payload)
    }
}

pub struct OpenAIConfigProcessor;
impl EngineConfigProcessor for OpenAIConfigProcessor {
    fn process_config(&self, config: &EngineConfig) -> Result<serde_json::Value> {
        debug!("OpenAIConfigProcessor::process_config");
        debug!("Config: {:#?}", config);

        let mut payload = json!({
            "model": config.parameters.get("modelName")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("Model not specified or not a string"))?,
            "messages": [], // This will be filled later with the actual request
        });

        // Handle temperature
        if let Some(temperature) = config.parameters.get("temperature") {
            if let Some(temp) = temperature.as_f64() {
                payload["temperature"] = json!(temp);
            }
        }

        // Handle max_tokens
        if let Some(max_tokens) = config.parameters.get("max_tokens") {
            if let Some(max_tokens_str) = max_tokens.as_str() {
                if let Ok(max_tokens_int) = max_tokens_str.parse::<u64>() {
                    payload["max_tokens"] = json!(max_tokens_int);
                }
            } else if let Some(max_tokens_int) = max_tokens.as_u64() {
                payload["max_tokens"] = json!(max_tokens_int);
            }
        }

        // Add optional parameters if they exist in the configuration
        //stream was removed from the list of optional parameters
        for &param in &[
            "frequency_penalty",
            "presence_penalty",
            "top_p",
            "n",
            "stop",
            "response_format",
        ] {
            if let Some(value) = config.parameters.get(param) {
                payload[param] = value.clone();
            }
        }

        debug!("OpenAI Payload: {:#?}", payload);
        Ok(payload)
    }
}

pub struct TextProcessor;
pub struct PdfProcessor;
pub struct DocxProcessor;
#[async_trait]
pub trait DocumentProcessor {
    async fn process(&self, file_path: &Path) -> Result<(String, Vec<String>)>;
}

#[async_trait]
impl DocumentProcessor for TextProcessor {
    async fn process(&self, file_path: &Path) -> Result<(String, Vec<String>)> {
        let mut file = File::open(file_path).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;
        Ok((content, vec![]))
    }
}

#[async_trait]
impl DocumentProcessor for PdfProcessor {
    async fn process(&self, file_path: &Path) -> Result<(String, Vec<String>)> {
        // Clone the PathBuf to move it into the closure
        debug!("PdfProcessor::process");
        let path_buf = file_path.to_path_buf();

        // Extract text from PDF
        let text = tokio::task::spawn_blocking(move || {
            debug!("PdfProcessor::process: Extracting text from PDF");
            extract_text(&path_buf)
        })
        .await??;

        // Extract metadata (you can expand this based on your needs)
        let mut metadata = Vec::new();
        debug!("PdfProcessor::process: Extracting metadata from PDF");

        // Add file name to metadata
        if let Some(file_name) = file_path.file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                metadata.push(format!("filename:{}", file_name_str));
            }
        }
        debug!("PdfProcessor::process: Metadata: {:#?}", metadata);
        // Add file size to metadata
        let file_size = tokio::fs::metadata(file_path).await?.len();

        debug!("PdfProcessor::process: File size: {}", file_size);
        metadata.push(format!("file_size:{}", file_size));

        // You can add more metadata extraction here, such as:
        // - Number of pages
        // - Author
        // - Creation date
        // - etc.

        Ok((text, metadata))
    }
}

#[async_trait]
impl DocumentProcessor for DocxProcessor {
    async fn process(&self, file_path: &Path) -> Result<(String, Vec<String>)> {
        let mut file = File::open(file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let file_path = file_path.to_owned();
        let file_size = buffer.len(); // Calculate buffer length here

        let content = tokio::task::spawn_blocking(move || -> Result<String> {
            // Instead of parsing DOCX, we'll just read the file as UTF-8 text
            // This won't work for actual DOCX files, but serves as a placeholder
            let content = String::from_utf8_lossy(&buffer).to_string();

            // Simulate paragraph separation
            let content = content.replace("\n", "\n\n");

            Ok(content)
        })
        .await??;

        let metadata = vec![
            format!(
                "filename:{}",
                file_path
                    .file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| "unknown".into())
            ),
            format!("filesize:{}", file_size), // Use file_size here instead of buffer.len()
            "filetype:docx".to_string(), // Adding this to maintain similarity with original function
        ];

        Ok((content, metadata))
    }
}
