//! Response formatting and output handling
//!
//! This module handles the formatting and display of responses from LLM engines,
//! including JSON output, colored output, and various formatting options.

use fluent_core::types::Response;
use serde_json;
use std::io;

/// Print a response with formatting options
pub fn print_response(response: &Response, response_time: f64) {
    print_response_with_options(response, response_time, &OutputOptions::default());
}

/// Output formatting options
#[derive(Debug, Clone)]
pub struct OutputOptions {
    pub json_format: bool,
    pub no_color: bool,
    pub verbose: bool,
    pub markdown: bool,
    pub show_usage: bool,
    pub show_cost: bool,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            json_format: false,
            no_color: false,
            verbose: false,
            markdown: false,
            show_usage: true,
            show_cost: true,
        }
    }
}

/// Print response with detailed formatting options
pub fn print_response_with_options(
    response: &Response,
    response_time: f64,
    options: &OutputOptions,
) {
    if options.json_format {
        print_json_response(response, response_time);
        return;
    }

    if options.markdown {
        print_markdown_response(response, response_time, options);
        return;
    }

    print_standard_response(response, response_time, options);
}

/// Print response in JSON format
fn print_json_response(response: &Response, response_time: f64) {
    let json_output = serde_json::json!({
        "content": response.content,
        "model": response.model,
        "usage": {
            "prompt_tokens": response.usage.prompt_tokens,
            "completion_tokens": response.usage.completion_tokens,
            "total_tokens": response.usage.total_tokens
        },
        "cost": {
            "prompt_cost": response.cost.prompt_cost,
            "completion_cost": response.cost.completion_cost,
            "total_cost": response.cost.total_cost
        },
        "response_time": response_time,
        "finish_reason": response.finish_reason
    });

    println!("{}", serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string()));
}

/// Print response in markdown format
fn print_markdown_response(response: &Response, response_time: f64, options: &OutputOptions) {
    println!("# AI Response\n");
    println!("**Model:** {}\n", response.model);
    
    if let Some(reason) = &response.finish_reason {
        println!("**Finish Reason:** {}\n", reason);
    }
    
    println!("## Content\n");
    println!("{}\n", response.content);
    
    if options.show_usage {
        println!("## Usage Statistics\n");
        println!("- **Prompt Tokens:** {}", response.usage.prompt_tokens);
        println!("- **Completion Tokens:** {}", response.usage.completion_tokens);
        println!("- **Total Tokens:** {}", response.usage.total_tokens);
        println!();
    }
    
    if options.show_cost {
        println!("## Cost Information\n");
        println!("- **Prompt Cost:** ${:.6}", response.cost.prompt_cost);
        println!("- **Completion Cost:** ${:.6}", response.cost.completion_cost);
        println!("- **Total Cost:** ${:.6}", response.cost.total_cost);
        println!();
    }
    
    println!("**Response Time:** {:.2}s", response_time);
}

/// Print response in standard format
fn print_standard_response(response: &Response, response_time: f64, options: &OutputOptions) {
    // Print content
    if options.no_color {
        println!("{}", response.content);
    } else {
        println!("\x1b[32m{}\x1b[0m", response.content);
    }

    if options.verbose {
        println!();
        print_separator(options.no_color);
        
        if options.no_color {
            println!("Model: {}", response.model);
        } else {
            println!("\x1b[36mModel:\x1b[0m {}", response.model);
        }

        if let Some(reason) = &response.finish_reason {
            if options.no_color {
                println!("Finish Reason: {}", reason);
            } else {
                println!("\x1b[36mFinish Reason:\x1b[0m {}", reason);
            }
        }

        if options.show_usage {
            if options.no_color {
                println!("Usage: {} prompt + {} completion = {} total tokens",
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                    response.usage.total_tokens);
            } else {
                println!("\x1b[36mUsage:\x1b[0m {} prompt + {} completion = {} total tokens",
                    response.usage.prompt_tokens,
                    response.usage.completion_tokens,
                    response.usage.total_tokens);
            }
        }

        if options.show_cost {
            if options.no_color {
                println!("Cost: ${:.6} (${:.6} prompt + ${:.6} completion)",
                    response.cost.total_cost,
                    response.cost.prompt_cost,
                    response.cost.completion_cost);
            } else {
                println!("\x1b[36mCost:\x1b[0m ${:.6} (${:.6} prompt + ${:.6} completion)",
                    response.cost.total_cost,
                    response.cost.prompt_cost,
                    response.cost.completion_cost);
            }
        }

        if options.no_color {
            println!("Response Time: {:.2}s", response_time);
        } else {
            println!("\x1b[36mResponse Time:\x1b[0m {:.2}s", response_time);
        }
    }
}

/// Print a separator line
fn print_separator(no_color: bool) {
    if no_color {
        println!("---");
    } else {
        println!("\x1b[90m---\x1b[0m");
    }
}

/// Print an error message with formatting
pub fn print_error(message: &str, no_color: bool) {
    if no_color {
        eprintln!("Error: {}", message);
    } else {
        eprintln!("\x1b[31mError:\x1b[0m {}", message);
    }
}

/// Print a warning message with formatting
pub fn print_warning(message: &str, no_color: bool) {
    if no_color {
        eprintln!("Warning: {}", message);
    } else {
        eprintln!("\x1b[33mWarning:\x1b[0m {}", message);
    }
}

/// Print an info message with formatting
pub fn print_info(message: &str, no_color: bool) {
    if no_color {
        println!("Info: {}", message);
    } else {
        println!("\x1b[34mInfo:\x1b[0m {}", message);
    }
}

/// Print a success message with formatting
pub fn print_success(message: &str, no_color: bool) {
    if no_color {
        println!("Success: {}", message);
    } else {
        println!("\x1b[32mSuccess:\x1b[0m {}", message);
    }
}

/// Write response to a file asynchronously
pub async fn write_response_to_file(
    response: &Response,
    file_path: &str,
    format: &str,
) -> Result<(), io::Error> {
    let content = match format {
        "json" => {
            let json_output = serde_json::json!({
                "content": response.content,
                "model": response.model,
                "usage": response.usage,
                "cost": response.cost,
                "finish_reason": response.finish_reason
            });
            serde_json::to_string_pretty(&json_output).unwrap_or_else(|_| "{}".to_string())
        }
        "markdown" => {
            format!(
                "# AI Response\n\n**Model:** {}\n\n## Content\n\n{}\n\n## Usage\n\n- Prompt Tokens: {}\n- Completion Tokens: {}\n- Total Tokens: {}\n\n## Cost\n\n- Total Cost: ${:.6}\n",
                response.model,
                response.content,
                response.usage.prompt_tokens,
                response.usage.completion_tokens,
                response.usage.total_tokens,
                response.cost.total_cost
            )
        }
        _ => response.content.clone(),
    };

    tokio::fs::write(file_path, content).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_core::types::{Usage, Cost};

    fn create_test_response() -> Response {
        Response {
            content: "Test response content".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            },
            model: "test-model".to_string(),
            finish_reason: Some("stop".to_string()),
            cost: Cost {
                prompt_cost: 0.001,
                completion_cost: 0.002,
                total_cost: 0.003,
            },
        }
    }

    #[test]
    fn test_output_options_default() {
        let options = OutputOptions::default();
        assert!(!options.json_format);
        assert!(!options.no_color);
        assert!(!options.verbose);
        assert!(!options.markdown);
        assert!(options.show_usage);
        assert!(options.show_cost);
    }

    #[tokio::test]
    async fn test_write_response_to_file() {
        let response = create_test_response();
        let temp_file = "/tmp/test_response.txt";

        let result = write_response_to_file(&response, temp_file, "text").await;
        assert!(result.is_ok());

        // Clean up
        let _ = tokio::fs::remove_file(temp_file).await;
    }
}
