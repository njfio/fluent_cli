
use std::future::Future;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::{Result, anyhow, Context};
use serde_json::{json, Value};
use log::debug;
use pdf_extract::extract_text;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use crate::config::EngineConfig;
use crate::neo4j_client::Neo4jClient;
use crate::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};


#[async_trait]

pub trait FileUpload: Send + Sync {
    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;
    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;
}



#[async_trait]
pub trait Engine: Send + Sync {

    fn execute<'a>(&'a self, request: &'a Request) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;

    fn upsert<'a>(&'a self, request: &'a UpsertRequest) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a>;

    fn get_neo4j_client(&self) -> Option<&Arc<Neo4jClient>>;

    fn get_session_id(&self) -> Option<String>;  // New method

    fn extract_content(&self, value: &Value) -> Option<ExtractedContent>;

    fn upload_file<'a>(&'a self, file_path: &'a Path) -> Box<dyn Future<Output = Result<String>> + Send + 'a>;
    fn process_request_with_file<'a>(&'a self, request: &'a Request, file_path: &'a Path) -> Box<dyn Future<Output = Result<Response>> + Send + 'a>;


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
                    },
                    "temperature" | "top_p" => {
                        if let Some(num) = value.as_str().and_then(|s| s.parse::<f64>().ok()) {
                            payload[key] = json!(num);
                        } else if let Some(num) = value.as_f64() {
                            payload[key] = json!(num);
                        }
                    },
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
        for &param in &["frequency_penalty", "presence_penalty", "top_p", "n", "stream", "stop"] {
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
        }).await??;

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


use docx;
use docx::{Docx, DocxFile};
use docx::document::{BodyContent, ParaContent, RunContent};

#[async_trait]
impl DocumentProcessor for DocxProcessor {
    async fn process(&self, file_path: &Path) -> Result<(String, Vec<String>)> {
        let file_path = file_path.to_owned();
        let file_path_clone = file_path.clone();

        // Read file contents asynchronously
        let mut file = tokio::fs::File::open(&file_path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let content = tokio::task::spawn_blocking(move || -> Result<String> {
            // Verify file size
            let file_size = buffer.len();
            if file_size == 0 {
                return Err(anyhow::anyhow!("DOCX file is empty"));
            }

            // Check ZIP signature
            if buffer.len() < 4 || &buffer[0..4] != b"PK\x03\x04" {
                return Err(anyhow::anyhow!("File does not have a valid ZIP signature"));
            }

            // Create a Cursor from our buffer
            let cursor = Cursor::new(buffer);

            let docx_file = DocxFile::from_reader(cursor)
                .map_err(|e| anyhow::anyhow!("Failed to open DOCX file: {:?}", e))?;

            // Use a more lenient parsing method
            let docx = match docx_file.parse() {
                Ok(doc) => doc,
                Err(e) => {
                    eprintln!("Warning: Failed to fully parse DOCX file: {:?}", e);
                    eprintln!("Attempting to extract available text...");
                    docx_file.parse()
                        .map_err(|e| anyhow::anyhow!("Failed to parse DOCX XML: {:?}", e))?
                }
            };

            let mut content = String::new();
            DocxProcessor::extract_text_from_docx(&docx, &mut content);

            Ok(content)
        }).await??;

        let metadata = vec![
            format!("filename:{}", file_path_clone.file_name().unwrap().to_string_lossy()),
            format!("filesize:{}", std::fs::metadata(&file_path_clone)?.len()),
        ];

        Ok((content, metadata))
    }
    }

impl DocxProcessor {
    fn extract_text_from_docx(docx: &Docx, content: &mut String) {
        if let Err(e) = Self::try_extract_text_from_docx(docx, content) {
            eprintln!("Warning: Error while extracting text: {:?}", e);
        }
    }

    fn try_extract_text_from_docx(docx: &Docx, content: &mut String) -> Result<()> {
        for child in &docx.document.body.content {
            match child {
                BodyContent::Para(paragraph) => {
                    for para_content in &paragraph.content {
                        match para_content {
                            ParaContent::Run(run) => {
                                for run_content in &run.content {
                                    if let RunContent::Text(text) = run_content {
                                        content.push_str(&text.text);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    content.push('\n');
                }
                _ => {}
            }
        }
        Ok(())
    }


    fn extract_text_from_document(document: &docx::document::Document, content: &mut String) {
        for child in &document.body.content {
            if let BodyContent::Para(paragraph) = child {
                Self::extract_text_from_paragraph(paragraph, content);
                content.push('\n');
            }
        }
    }

    fn extract_text_from_paragraph(paragraph: &docx::document::Para, content: &mut String) {
        for child in &paragraph.content {
            if let ParaContent::Run(run) = child {
                Self::extract_text_from_run(run, content);
            }
        }
    }

    fn extract_text_from_run(run: &docx::document::Run, content: &mut String) {
        for child in &run.content {
            if let RunContent::Text(text) = child {
                content.push_str(&text.text);
            }
        }
    }
}