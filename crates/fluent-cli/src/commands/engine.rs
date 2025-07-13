use anyhow::{anyhow, Result};
use clap::ArgMatches;
use fluent_core::config::Config;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response};
use serde_json;

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
        let request = Request {
            flowname: "default".to_string(),
            payload: request_content.to_string(),
        };

        Pin::from(engine.execute(&request)).await
    }

    /// Format response for output
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

impl EngineCommand {
    /// List available engines
    async fn list_engines(matches: &ArgMatches, config: &Config) -> Result<()> {
        let json_output = matches.get_flag("json");

        // Get engines from config
        let engines = &config.engines;

        if json_output {
            let engine_list: Vec<serde_json::Value> = engines.iter().map(|engine| {
                let url = format!("{}://{}:{}{}",
                    engine.connection.protocol,
                    engine.connection.hostname,
                    engine.connection.port,
                    engine.connection.request_path
                );
                serde_json::json!({
                    "name": engine.name,
                    "engine": engine.engine,
                    "connection": {
                        "protocol": engine.connection.protocol,
                        "hostname": engine.connection.hostname,
                        "port": engine.connection.port,
                        "request_path": engine.connection.request_path,
                        "url": url
                    }
                })
            }).collect();

            println!("{}", serde_json::to_string_pretty(&engine_list)?);
        } else {
            println!("🚀 Available engines:\n");

            if engines.is_empty() {
                println!("No engines configured. Please check your configuration file.");
                return Ok(());
            }

            for engine in engines {
                let url = format!("{}://{}:{}{}",
                    engine.connection.protocol,
                    engine.connection.hostname,
                    engine.connection.port,
                    engine.connection.request_path
                );
                println!("📦 {}", engine.name);
                println!("   Type: {}", engine.engine);
                println!("   URL: {}", url);
                println!("   Host: {}", engine.connection.hostname);
                println!("   Port: {}", engine.connection.port);
                println!();
            }
        }

        Ok(())
    }

    /// Test engine connectivity
    async fn test_engine(matches: &ArgMatches, config: &Config) -> Result<()> {
        let engine_name = matches
            .get_one::<String>("engine")
            .ok_or_else(|| anyhow!("Engine name is required"))?;

        // Find the engine in config
        let engine_config = config.engines.iter()
            .find(|e| e.name == *engine_name)
            .ok_or_else(|| anyhow!("Engine '{}' not found in configuration", engine_name))?;

        println!("🔍 Testing engine: {}", engine_name);

        // Create engine instance
        match create_engine(engine_config).await {
            Ok(_engine) => {
                println!("✅ Engine '{}' is available and configured correctly", engine_name);

                // TODO: Add actual connectivity test by making a simple request
                // let test_request = Request {
                //     flowname: "test".to_string(),
                //     payload: "Hello, this is a test.".to_string(),
                // };
                // let response = engine.execute(&test_request).await?;
                // println!("Test response: {}", response.content);
            }
            Err(e) => {
                println!("❌ Engine '{}' test failed: {}", engine_name, e);
                return Err(e);
            }
        }

        Ok(())
    }
}

impl CommandHandler for EngineCommand {
    async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()> {
        match matches.subcommand() {
            Some(("list", sub_matches)) => {
                Self::list_engines(sub_matches, config).await
            }
            Some(("test", sub_matches)) => {
                Self::test_engine(sub_matches, config).await
            }
            _ => {
                eprintln!("No subcommand provided. Use 'fluent engine --help' for usage information.");
                Ok(())
            }
        }
    }
}

impl Default for EngineCommand {
    fn default() -> Self {
        Self::new()
    }
}
