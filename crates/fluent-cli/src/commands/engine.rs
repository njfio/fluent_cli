use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_core::config::Config;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response};

use fluent_engines::create_engine;
use std::path::Path;
use std::pin::Pin;

use super::{CommandHandler, CommandResult};

/// Engine command handler for direct LLM interactions
pub struct EngineCommand;

impl EngineCommand {
    pub fn new() -> Self {
        Self
    }

    /// Validate request payload
    fn validate_request_payload(payload: &str, _context: &str) -> Result<String> {
        if payload.trim().is_empty() {
            return Err(anyhow!("Request payload cannot be empty"));
        }

        // Basic validation - in production, add more sophisticated checks
        if payload.len() > 100_000 {
            return Err(anyhow!("Request payload too large (max 100KB)"));
        }

        Ok(payload.to_string())
    }

    /// Process request with file upload
    async fn process_request_with_file(
        engine: &dyn Engine,
        request_content: &str,
        file_path: &str,
    ) -> Result<Response> {
        let file_id = Pin::from(engine.upload_file(Path::new(file_path))).await?;
        let request = Request {
            flowname: "default".to_string(),
            payload: format!("{}\n\nFile ID: {}", request_content, file_id),
        };

        Pin::from(engine.execute(&request)).await
    }

    /// Process simple request
    async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
        let request = Request {
            flowname: "default".to_string(),
            payload: request_content.to_string(),
        };

        Pin::from(engine.execute(&request)).await
    }

    /// Format response for output
    fn format_response(response: &Response, parse_code: bool, markdown: bool) -> String {
        let mut output = response.content.clone();

        if parse_code {
            // Extract and highlight code blocks
            output = Self::extract_code_blocks(&output);
        }

        if markdown {
            // Format as markdown (simplified)
            output = format!("# Response\n\n{}", output);
        }

        output
    }

    /// Extract code blocks from response
    fn extract_code_blocks(content: &str) -> String {
        // Simplified code block extraction
        let mut result = String::new();
        let mut in_code_block = false;

        for line in content.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    in_code_block = false;
                } else {
                    in_code_block = true;
                    let language = line.trim_start_matches("```").trim();
                    if !language.is_empty() {
                        result.push_str(&format!("\n--- {} Code Block ---\n", language));
                    } else {
                        result.push_str("\n--- Code Block ---\n");
                    }
                }
            } else if in_code_block {
                result.push_str(line);
                result.push('\n');
            }
        }

        if result.is_empty() {
            content.to_string()
        } else {
            result
        }
    }

    /// Execute engine request with all options
    async fn execute_engine_request(
        engine_name: &str,
        request: &str,
        config: &Config,
        matches: &ArgMatches,
    ) -> Result<CommandResult> {
        // Find engine configuration
        let engine_config = config
            .engines
            .iter()
            .find(|e| e.name == engine_name)
            .ok_or_else(|| anyhow!("Engine '{}' not found in configuration", engine_name))?;

        // Create engine
        let engine = create_engine(engine_config).await?;

        // Get additional options
        let upload_file = matches.get_one::<String>("upload-image-file");
        let parse_code = matches.get_flag("parse-code");
        let markdown = matches.get_flag("markdown");

        // Validate request
        let validated_request = Self::validate_request_payload(request, "engine request")?;

        // Execute request
        let response = if let Some(file_path) = upload_file {
            Self::process_request_with_file(&*engine, &validated_request, file_path).await?
        } else {
            Self::process_request(&*engine, &validated_request).await?
        };

        // Format output
        let formatted_output = Self::format_response(&response, parse_code, markdown);

        println!("{}", formatted_output);

        Ok(CommandResult::success_with_data(serde_json::json!({
            "engine": engine_name,
            "response": response,
            "formatted_output": formatted_output
        })))
    }
}

impl CommandHandler for EngineCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        // Get engine name and request from arguments
        let engine_name = matches
            .get_one::<String>("engine")
            .ok_or_else(|| anyhow!("Engine name is required"))?;

        let request = matches
            .get_one::<String>("request")
            .ok_or_else(|| anyhow!("Request is required"))?;

        // Execute the request
        let result = Self::execute_engine_request(engine_name, request, config, matches).await?;

        if !result.success {
            if let Some(message) = result.message {
                eprintln!("Engine execution failed: {}", message);
            }
            std::process::exit(1);
        }

        Ok(())
    }
}

impl Default for EngineCommand {
    fn default() -> Self {
        Self::new()
    }
}
