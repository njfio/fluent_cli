use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use anyhow::{anyhow, Error};
use clap::ArgMatches;
use tokio::fs;
use fluent_core::config::{Config, EngineConfig, Neo4jConfig};
use fluent_core::neo4j_client::Neo4jClient;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info, warn};
use regex::Regex;
use serde_json::{json, Value};
use fluent_core::spinner_configuration::SpinnerConfig;
use fluent_core::traits::{Engine};
use fluent_core::types::Request;
use fluent_engines::anthropic::AnthropicEngine;
use fluent_engines::cohere::CohereEngine;
use fluent_engines::google_gemini::GoogleGeminiEngine;
use fluent_engines::groqlpu::GroqLPUEngine;
use fluent_engines::openai::OpenAIEngine;
use fluent_engines::perplexity::PerplexityEngine;

pub mod cli {
    use std::pin::Pin;
    use std::io::{self, Read};
    use std::fs;
    use std::env;
    use clap::{Command, Arg, ArgAction, ArgMatches, CommandFactory, ValueEnum, ValueHint, value_parser};
    use clap_complete::{generate, Generator, Shell};
    use fluent_core::config::{load_config, Config, EngineConfig};
    use fluent_engines::openai::OpenAIEngine;
    use fluent_engines::anthropic::AnthropicEngine;
    use fluent_core::traits::Engine;
    use fluent_core::types::{Request, Response, UpsertRequest};
    use anyhow::{Result, anyhow, Error};
    use std::collections::{HashMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use clap::builder::{PossibleValue, PossibleValuesParser};
    use owo_colors::OwoColorize;
    use std::io::IsTerminal;
    use indicatif::{ProgressBar, ProgressStyle};

    use log::{debug, error, info};
    use serde_json::Value;
    use tokio::io::AsyncReadExt;
    use tokio::process;
    use tokio::time::Instant;
    use uuid::Uuid;
    use fluent_core::neo4j_client::{InteractionStats, Neo4jClient};
    use fluent_core::output_processor::{format_markdown, MarkdownFormatter, OutputProcessor};
    use fluent_engines::cohere::CohereEngine;
    use fluent_engines::dalle::DalleEngine;
    use fluent_engines::flowise_chain::FlowiseChainEngine;
    use fluent_engines::google_gemini::GoogleGeminiEngine;
    use fluent_engines::groqlpu::GroqLPUEngine;
    use fluent_engines::imagepro::ImagineProEngine;
    use fluent_engines::langflow::LangflowEngine;
    use fluent_engines::perplexity::PerplexityEngine;
    use fluent_engines::webhook::WebhookEngine;
    use fluent_engines::leonardoai::LeonardoAIEngine;
    use fluent_engines::mistral::MistralEngine;
    use fluent_engines::pipeline_executor::{FileStateStore, Pipeline, PipelineExecutor};
    use fluent_engines::stabilityai::StabilityAIEngine;
    use crate::{create_engine, create_llm_engine, generate_and_execute_cypher};


    fn parse_key_value_pair(s: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = s.splitn(2, '=').collect();
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }

    pub struct CliState {
        pub command: Command,
        pub parameters: Vec<String>,
    }

    fn read_config_file(path: &str) -> Result<(Vec<String>, HashSet<String>)> {
        let config_str = fs::read_to_string(path)?;
        let config: Value = serde_json::from_str(&config_str)?;

        let engines = config["engines"]
            .as_array()
            .ok_or_else(|| anyhow!("No engines found in configuration"))?
            .iter()
            .filter_map(|engine| engine["name"].as_str().map(String::from))
            .collect::<Vec<String>>();

        let mut parameters = HashSet::new();
        for engine in config["engines"].as_array().unwrap() {
            if let Some(params) = engine["parameters"].as_object() {
                for key in params.keys() {
                    parameters.insert(key.clone());
                }
            }
        }

        Ok((engines, parameters))
    }

    async fn process_request_with_file(engine: &dyn Engine, request_content: &str, file_path: &str) -> Result<Response> {
        let file_id = Pin::from(engine.upload_file(Path::new(file_path))).await?;
        println!("File uploaded successfully. File ID: {}", file_id);

        let request = Request {
            flowname: "default".to_string(),
            payload: format!("File ID: {}. {}", file_id, request_content),
        };

        Pin::from(engine.execute(&request)).await
    }


    async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
        let request = Request {
            flowname: "default".to_string(),
            payload: request_content.to_string(),
        };

        Pin::from(engine.execute(&request)).await    }

    fn print_response(response: &Response, response_time: f64) {
        println!("Response: {}", response.content);
        println!("Model: {}", response.model);
        println!("Usage:");
        println!("  Prompt tokens: {}", response.usage.prompt_tokens);
        println!("  Completion tokens: {}", response.usage.completion_tokens);
        println!("  Total tokens: {}", response.usage.total_tokens);
        println!("  Response time: {:.2} seconds", response_time);
        if let Some(reason) = &response.finish_reason {
            println!("Finish reason: {}", reason);
        }
    }


    pub fn build_cli() -> Command {
        Command::new("Fluent CLI")
            .version("2.0")
            .author("Your Name <your.email@example.com>")
            .about("A powerful CLI for interacting with various AI engines")
            .arg(Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .required(false))
            .arg(Arg::new("engine")
                .help("The engine to use (openai or anthropic)")
                .required(true))
            .arg(Arg::new("request")
                .help("The request to process")
                .required(false))
            .arg(Arg::new("override")
                .short('o')
                .long("override")
                .value_name("KEY=VALUE")
                .help("Override configuration values")
                .action(ArgAction::Append)
                .num_args(1..))
            .arg(Arg::new("additional-context-file")
                .long("additional-context-file")
                .short('a')
                .help("Specifies a file from which additional request context is loaded")
                .action(ArgAction::Set)
                .value_hint(clap::ValueHint::FilePath)
                .required(false))
            .arg(Arg::new("upsert")
                .long("upsert")
                .help("Enables upsert mode")
                .action(ArgAction::SetTrue)
                .conflicts_with("request"))
            .arg(Arg::new("input")
                .long("input")
                .short('i')
                .value_name("FILE")
                .help("Input file or directory to process (required for upsert)")
                .required(false))
            .arg(Arg::new("metadata")
                .long("metadata")
                .short('t')
                .value_name("TERMS")
                .help("Comma-separated list of metadata terms (for upsert)")
                .required(false))
            .arg(Arg::new("upload-image-file")
                .short('l')
                .long("upload_image_file")
                .value_name("FILE")
                .help("Upload a media file")
                .action(ArgAction::Set)
                .required(false))
            .arg(Arg::new("download-media")
                .short('d')
                .long("download-media")
                .value_name("DIR")
                .help("Download media files from the output")
                .action(ArgAction::Set)
                .required(false))
            .arg(Arg::new("parse-code")
                .short('p')
                .long("parse-code")
                .help("Parse and display code blocks from the output")
                .action(ArgAction::SetTrue))
            .arg(Arg::new("execute-output")
                .short('x')
                .long("execute-output")
                .help("Execute code blocks from the output")
                .action(ArgAction::SetTrue))
            .arg(Arg::new("markdown")
                .short('m')
                .long("markdown")
                .help("Format output as markdown")
                .action(ArgAction::SetTrue))
            .arg(Arg::new("generate-cypher")
                .long("generate-cypher")
                .value_name("QUERY")
                .help("Generate and execute a Cypher query based on the given string")
                .action(ArgAction::Set)
                .required(false))
            .subcommand(Command::new("pipeline")
                .about("Execute a pipeline")
                .arg(Arg::new("file")
                    .short('f')
                    .long("file")
                    .help("The YAML file containing the pipeline definition")
                    .required(true))
                .arg(Arg::new("input")
                    .short('i')
                    .long("input")
                    .help("The input for the pipeline")
                    .required(true))
                .arg(Arg::new("force_fresh")
                    .long("force-fresh")
                    .help("Force a fresh execution of the pipeline")
                    .action(ArgAction::SetTrue)))
    }

    fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
        debug!("Printing completions for {}", cmd.get_name());
        generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    }

    async fn get_neo4j_query_llm(config: &Config) -> Option<(Box<dyn Engine>, &EngineConfig)> {
        let neo4j_config = config.engines.iter().find(|e| e.engine == "neo4j")?;
        let query_llm = neo4j_config.neo4j.as_ref()?.query_llm.as_ref()?;
        let llm_config = config.engines.iter().find(|e| e.name == query_llm.to_string())?;
        let engine = create_llm_engine(llm_config).await.ok()?;
        Some((engine, llm_config))
    }


    pub async fn run() -> Result<()> {
        let matches = build_cli().get_matches();

        let _: Result<(), Error> = match matches.subcommand() {
            Some(("pipeline", sub_matches)) => {
                let pipeline_file = sub_matches.get_one::<String>("file").unwrap();
                let input = sub_matches.get_one::<String>("input").unwrap();
                let force_fresh = sub_matches.get_flag("force_fresh");
                debug!("Force fresh: {}", force_fresh);

                let pipeline: Pipeline = serde_yaml::from_str(&std::fs::read_to_string(pipeline_file)?)?;
                let state_store_dir = PathBuf::from("./pipeline_states");
                tokio::fs::create_dir_all(&state_store_dir).await?;

                let state_store = FileStateStore { directory: state_store_dir };
                let executor = PipelineExecutor::new(state_store);

                let output = executor.execute(&pipeline, input, force_fresh).await?;
                eprintln!("Pipeline execution result:\n");
                println!("{}", output);

                std::process::exit(0);// Exit immediately after successful pipeline execution
            },
            // ... other commands ...
            _ => Ok(()), // Default case, do nothing
        };

        let config_path = matches.get_one::<String>("config")
            .map(|s| s.to_string())
            .or_else(|| env::var("FLUENT_CLI_V2_CONFIG_PATH").ok())
            .ok_or_else(|| anyhow!("No config file specified and FLUENT_CLI_V2_CONFIG_PATH environment variable not set"))?;

        let engine_name = matches.get_one::<String>("engine").unwrap();

        let overrides: HashMap<String, String> = matches.get_many::<String>("override")
            .map(|values| values.filter_map(|s| parse_key_value_pair(s)).collect())
            .unwrap_or_default();

        let config = load_config(&config_path, engine_name, &overrides)?;
        let spinner_config = config.engines[0].spinner.clone().unwrap_or_default();
        let pb = ProgressBar::new_spinner();
        let engine_config = &config.engines[0];
        let start_time = Instant::now();

        let spinner_style = ProgressStyle::default_spinner()
            .tick_chars(&spinner_config.frames)
            .template("{spinner:.green} {msg}")
            .unwrap();

        pb.set_style(spinner_style);
        pb.set_message(format!("Processing {} request...", engine_name));
        pb.enable_steady_tick(Duration::from_millis(spinner_config.interval));
        pb.set_length(100);



        if let Some(cypher_query) = matches.get_one::<String>("generate-cypher") {
            let neo4j_config = engine_config.neo4j.as_ref()
                .ok_or_else(|| anyhow!("Neo4j configuration not found in the engine config"))?;

            let query_llm_name = neo4j_config.query_llm.as_ref()
                .ok_or_else(|| anyhow!("No query LLM specified for Neo4j"))?;

            // Load the configuration for the query LLM
            let query_llm_config = load_config(&config_path, query_llm_name, &HashMap::new())?;
            let query_llm_engine_config = &query_llm_config.engines[0];

            let query_llm_engine = create_llm_engine(query_llm_engine_config).await?;

            let cypher_result = generate_and_execute_cypher(
                neo4j_config,
                query_llm_engine_config,
                cypher_query,
                &*query_llm_engine
            ).await?;

            if engine_config.engine == "neo4j" {
                println!("{}", cypher_result);
            } else {
                let engine: Box<dyn Engine> = create_engine(engine_config).await?;

                let max_tokens = engine_config.parameters.get("max_tokens")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);

                let user_request = matches.get_one::<String>("request")
                    .map(|s| s.to_string())
                    .unwrap_or_else(String::new);

                let mut combined_request = format!(
                    "Cypher query: {}\n\nCypher result:\n{}\n\nBased on the above Cypher query and its result, please provide an analysis or answer the following question: {}",
                    cypher_query, cypher_result, user_request
                );

                // Truncate the combined request if it exceeds the max tokens
                if max_tokens > 0 && combined_request.len() > max_tokens as usize {
                    combined_request.truncate(max_tokens as usize);
                    combined_request += "... [truncated]";
                }
                info!("Combined request: {}", combined_request);
                let request = Request {
                    flowname: engine_name.to_string(),
                    payload: combined_request,
                };

                let response = Pin::from(engine.execute(&request)).await?;
                let mut output = response.content.clone();

                if let Some(download_dir) = matches.get_one::<String>("download-media") {
                    let download_path = PathBuf::from(download_dir);
                    OutputProcessor::download_media_files(&response.content, &download_path).await?;
                }

                if matches.get_flag("parse-code") {
                    debug!("Parsing code blocks");
                    let code_blocks = OutputProcessor::parse_code(&output);
                    debug!("Code blocks: {:?}", code_blocks);
                    output = code_blocks.join("\n\n");
                }

                if matches.get_flag("execute-output") {
                    debug!("Executing output code");
                    debug!("Attempting to execute : {}", output);
                    output = OutputProcessor::execute_code(&output).await?;
                }

                if matches.get_flag("markdown") {
                    debug!("Formatting output as markdown");
                    //output = format_markdown(&output);
                }

                let response_time = start_time.elapsed().as_secs_f64();

                if let Some(neo4j_client) = engine.get_neo4j_client() {
                    let session_id = engine.get_session_id()
                        .unwrap_or_else(|| Uuid::new_v4().to_string());

                    let stats = InteractionStats {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: response.usage.total_tokens,
                        response_time,
                        finish_reason: response.finish_reason.clone().unwrap_or_else(|| "unknown".to_string()),
                    };

                    debug!("Attempting to create interaction in Neo4j");
                    debug!("Using session ID: {}", session_id);
                    match neo4j_client.create_interaction(
                        &session_id,
                        &request.payload,
                        &response.content,
                        &response.model,
                        &stats
                    ).await {
                        Ok(interaction_id) => debug!("Successfully created interaction with id: {}", interaction_id),
                        Err(e) => error!("Failed to create interaction in Neo4j: {:?}", e),
                    }
                } else {
                    debug!("Neo4j client not available, skipping interaction logging");
                }

                pb.finish_and_clear();
                eprintln!();
                println!("{}", output);

                let use_colors = std::io::stderr().is_terminal();
                let response_time_str = format!("{:.2}s", response_time);

                eprintln!(
                    "{} | {} | Time: {} | Usage: {}↑ {}↓ {}Σ | {}\n",
                    spinner_config.success_symbol,
                    if use_colors { response.model.cyan().to_string() } else { response.model },
                    if use_colors { response_time_str.bright_blue().to_string() } else { response_time_str },
                    if use_colors { response.usage.prompt_tokens.to_string().yellow().to_string() } else { response.usage.prompt_tokens.to_string() },
                    if use_colors { response.usage.completion_tokens.to_string().yellow().to_string() } else { response.usage.completion_tokens.to_string() },
                    if use_colors { response.usage.total_tokens.to_string().yellow().to_string() } else { response.usage.total_tokens.to_string() },
                    if use_colors { response.finish_reason.as_deref().unwrap_or("No finish reason").italic().to_string() } else { response.finish_reason.as_deref().unwrap_or("No finish reason").to_string() }
                );
            }
        } else if matches.get_flag("upsert") {
            debug!("Upsert mode enabled");
            handle_upsert(engine_config, &matches).await?;
        } else {
            debug!("No mode specified, defaulting to interactive mode");
            let request = matches.get_one::<String>("request").unwrap();

            let engine: Box<dyn Engine> = match engine_config.engine.as_str() {
                "anthropic" => Box::new(AnthropicEngine::new(engine_config.clone()).await?),
                "openai" => Box::new(OpenAIEngine::new(engine_config.clone()).await?),
                "cohere" => Box::new(CohereEngine::new(engine_config.clone()).await?),
                "google_gemini" => Box::new(GoogleGeminiEngine::new(engine_config.clone()).await?),
                "mistral" => Box::new(MistralEngine::new(engine_config.clone()).await?),
                "groq_lpu" => Box::new(GroqLPUEngine::new(engine_config.clone()).await?),
                "perplexity" => Box::new(PerplexityEngine::new(engine_config.clone()).await?),
                "webhook" => Box::new(WebhookEngine::new(engine_config.clone()).await?),
                "flowise_chain" => Box::new(FlowiseChainEngine::new(engine_config.clone()).await?),
                "langflow_chain" => Box::new(LangflowEngine::new(engine_config.clone()).await?),
                "dalle" => Box::new(DalleEngine::new(engine_config.clone()).await?),
                "stabilityai" => {
                    let mut engine = Box::new(StabilityAIEngine::new(engine_config.clone()).await?);
                    if let Some(download_dir) = matches.get_one::<String>("download-media") {
                        engine.set_download_dir(download_dir.to_string());
                    }
                    engine
                },
                "leonardo_ai" => Box::new(LeonardoAIEngine::new(engine_config.clone()).await?),
                "imagine_pro" => {
                    let mut engine = Box::new(ImagineProEngine::new(engine_config.clone()).await?);
                    if let Some(download_dir) = matches.get_one::<String>("download-media") {
                        engine.set_download_dir(download_dir.to_string());
                    }
                    engine
                }
                _ => return Err(anyhow!("Unsupported engine: {}", engine_config.engine)),
            };

            // Read context from stdin if available
            let mut context = String::new();
            if !atty::is(atty::Stream::Stdin) {
                tokio::io::stdin().read_to_string(&mut context).await?;
            }

            // Read additional context from file if provided
            let mut file_contents = String::new();
            if let Some(file_path) = matches.get_one::<String>("additional-context-file") {
                file_contents = fs::read_to_string(file_path)?;
            }

            // Combine all inputs
            let mut combined_request_parts = Vec::new();
            // Always add the request first
            combined_request_parts.push(request.trim().to_string());
            // Add context if it's not empty
            if !context.trim().is_empty() {
                combined_request_parts.push(format!("Context:\n{}", context.trim()));
            }
            // Add file contents if it's not empty
            if !file_contents.trim().is_empty() {
                combined_request_parts.push(format!("Additional Context:\n{}", file_contents.trim()));
            }
            // Join all parts with a separator
            let combined_request = combined_request_parts.join("\n\n----\n\n");
            debug!("Combined Request:\n{}", combined_request);

            let request = Request {
                flowname: engine_name.to_string(),
                payload: combined_request,
            };
            debug!("Combined Request: {:?}", request);


            let response = if let Some(file_path) = matches.get_one::<String>("upload-image-file") {
                debug!("Processing request with file: {}", file_path);
                pb.set_message("Processing request with file...");
                Pin::from(engine.process_request_with_file(&request, Path::new(file_path))).await?
            } else {
                pb.set_message("Executing request...");
                Pin::from(engine.execute(&request)).await?
            };

            let mut output = response.content.clone();

            if let Some(download_dir) = matches.get_one::<String>("download-media") {
                let download_path = PathBuf::from(download_dir);
                OutputProcessor::download_media_files(&response.content, &download_path).await?;
            }

            if matches.get_flag("parse-code") {
                debug!("Parsing code blocks");
                let code_blocks = OutputProcessor::parse_code(&output);
                debug!("Code blocks: {:?}", code_blocks);
                output = code_blocks.join("\n\n");
            }

            if matches.get_flag("execute-output") {
                debug!("Executing output code");
                debug!("Attempting to execute : {}", output);
                output = OutputProcessor::execute_code(&output).await?;
            }

            if matches.get_flag("markdown") {
                debug!("Formatting output as markdown");
                //output = format_markdown(&output);
            }

            let response_time = start_time.elapsed().as_secs_f64();

            if let Some(neo4j_client) = engine.get_neo4j_client() {
                let session_id = engine.get_session_id()
                    .unwrap_or_else(|| Uuid::new_v4().to_string());

                let stats = InteractionStats {
                    prompt_tokens: response.usage.prompt_tokens,
                    completion_tokens: response.usage.completion_tokens,
                    total_tokens: response.usage.total_tokens,
                    response_time,
                    finish_reason: response.finish_reason.clone().unwrap_or_else(|| "unknown".to_string()),
                };

                debug!("Attempting to create interaction in Neo4j");
                debug!("Using session ID: {}", session_id);
                match neo4j_client.create_interaction(
                    &session_id,
                    &request.payload,
                    &response.content,
                    &response.model,
                    &stats
                ).await {
                    Ok(interaction_id) => debug!("Successfully created interaction with id: {}", interaction_id),
                    Err(e) => error!("Failed to create interaction in Neo4j: {:?}", e),
                }
            } else {
                debug!("Neo4j client not available, skipping interaction logging");
            }

            pb.finish_and_clear();
            eprintln!();
            println!("{}", output);

            let use_colors = std::io::stderr().is_terminal();
            let response_time_str = format!("{:.2}s", response_time);

            eprintln!(
                "{} | {} | Time: {} | Usage: {}↑ {}↓ {}Σ | {}\n",
                spinner_config.success_symbol,
                if use_colors { response.model.cyan().to_string() } else { response.model },
                if use_colors { response_time_str.bright_blue().to_string() } else { response_time_str },
                if use_colors { response.usage.prompt_tokens.to_string().yellow().to_string() } else { response.usage.prompt_tokens.to_string() },
                if use_colors { response.usage.completion_tokens.to_string().yellow().to_string() } else { response.usage.completion_tokens.to_string() },
                if use_colors { response.usage.total_tokens.to_string().yellow().to_string() } else { response.usage.total_tokens.to_string() },
                if use_colors { response.finish_reason.as_deref().unwrap_or("No finish reason").italic().to_string() } else { response.finish_reason.as_deref().unwrap_or("No finish reason").to_string() }
            );
        }

        Ok(())
    }


    async fn handle_upsert(engine_config: &EngineConfig, matches: &ArgMatches) -> Result<()> {
        if let Some(neo4j_config) = &engine_config.neo4j {
            let neo4j_client = Neo4jClient::new(neo4j_config).await?;

            let input = matches.get_one::<String>("input")
                .ok_or_else(|| anyhow!("Input is required for upsert mode"))?;
            let metadata = matches.get_one::<String>("metadata")
                .map(|s| s.split(',').map(String::from).collect::<Vec<String>>())
                .unwrap_or_default();

            let input_path = Path::new(input);
            if input_path.is_file() {
                let document_id = neo4j_client.upsert_document(input_path, &metadata).await?;
                eprintln!("Uploaded document with ID: {}. Embeddings and chunks created.", document_id);
            } else if input_path.is_dir() {
                let mut uploaded_count = 0;
                for entry in fs::read_dir(input_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let document_id = neo4j_client.upsert_document(&path, &metadata).await?;
                        eprintln!("Uploaded document {} with ID: {}. Embeddings and chunks created.", path.display(), document_id);
                        uploaded_count += 1;
                    }
                }
                eprintln!("Uploaded {} documents with embeddings and chunks", uploaded_count);
            } else {
                return Err(anyhow!("Input is neither a file nor a directory"));
            }

            if let Ok(stats) = neo4j_client.get_document_statistics().await {
                eprintln!("\nDocument Statistics:");
                eprintln!("Total documents: {}", stats.document_count);
                eprintln!("Average content length: {:.2}", stats.avg_content_length);
                eprintln!("Total chunks: {}", stats.chunk_count);
                eprintln!("Total embeddings: {}", stats.embedding_count);
            }
        } else {
            return Err(anyhow!("Neo4j configuration not found for this engine"));
        }

        Ok(())
    }


    async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
        // Use the configured LLM to generate a Cypher query
        let llm_request = Request {
            flowname: "cypher_generation".to_string(),
            payload: format!("Generate a Cypher query for Neo4j based on this request: {}", query),
        };
        debug!("Sending request to LLM engine: {:?}", llm_request);
        let llm_engine: Box<dyn Engine> = match config.engine.as_str() {
            "openai" => Box::new(OpenAIEngine::new(config.clone()).await?),
            "anthropic" => Box::new(AnthropicEngine::new(config.clone()).await?),
            // Add other LLM engines as needed
            _ => return Err(anyhow!("Unsupported LLM engine for Cypher generation")),
        };


        let response = Pin::from(llm_engine.execute(&llm_request)).await?;


        debug!("Response from LLM engine: {:?}", response);
        Ok(response.content)
    }
}


async fn get_neo4j_query_llm(config: &Config) -> Option<Box<dyn Engine>> {
    let neo4j_config = config.engines.iter().find(|e| e.engine == "neo4j")?;

    // Extract the query_llm from the neo4j configuration
    let query_llm = neo4j_config.neo4j.as_ref()?.query_llm.as_ref()?;

    // Find the engine configuration for the specified query_llm
    let llm_config = config.engines.iter().find(|e| e.name.as_str() == query_llm)?;

    // Create and return the LLM engine
    create_llm_engine(llm_config).await.ok()
}

async fn generate_and_execute_cypher(
    neo4j_config: &Neo4jConfig,
    llm_config: &EngineConfig,
    query_string: &str,
    llm_engine: &dyn Engine
) -> Result<String, Error> {
    debug!("Generating Cypher query using LLM");
    debug!("Neo4j configuration: {:#?}", neo4j_config);
    let neo4j_client = Neo4jClient::new(neo4j_config).await?;
    debug!("Neo4j client created");

    // Fetch the database schema
    let schema = neo4j_client.get_database_schema().await?;
    debug!("Database schema: {:#?}", schema);

    // Generate Cypher query using LLM
    let cypher_request = Request {
        flowname: "generate_cypher".to_string(),
        payload: format!(
            "Given the following database schema:\n\n{}\n\nGenerate a Cypher query for Neo4j based on this request: {}",
            schema, query_string
        ),
    };
    //info!("Sending request to LLM engine: {:?}", cypher_request);
    let cypher_response = Pin::from(llm_engine.execute(&cypher_request)).await?;
    let cypher_query = extract_cypher_query(&cypher_response.content)?;

    // Execute the Cypher query
    let cypher_result = neo4j_client.execute_cypher(&cypher_query).await?;
    debug!("Cypher result: {:?}", cypher_result);

    // Format the result based on the output format
    Ok(format_as_csv(&cypher_result))
}


fn extract_cypher_query(content: &str) -> Result<String, Error> {
    // First, try to extract content between triple backticks
    let backtick_re = Regex::new(r"```(?:cypher)?\s*([\s\S]*?)\s*```").unwrap();
    if let Some(captures) = backtick_re.captures(content) {
        if let Some(query) = captures.get(1) {
            let extracted = query.as_str().trim();
            if is_valid_cypher(extracted) {
                return Ok(extracted.to_string());
            }
        }
    }

    // If not found, look for common Cypher keywords to identify the query
    let cypher_re = Regex::new(r"(?i)(MATCH|CREATE|MERGE|DELETE|REMOVE|SET|RETURN)[\s\S]+").unwrap();
    if let Some(captures) = cypher_re.captures(content) {
        if let Some(query) = captures.get(0) {
            let extracted = query.as_str().trim();
            if is_valid_cypher(extracted) {
                return Ok(extracted.to_string());
            }
        }
    }

    // If still not found, return an error
    Err(anyhow!("No valid Cypher query found in the content"))
}

fn is_valid_cypher(query: &str) -> bool {
    // Basic validation: check if the query contains common Cypher clauses
    let valid_clauses = ["MATCH", "CREATE", "MERGE", "DELETE", "REMOVE", "SET", "RETURN", "WITH", "WHERE"];
    valid_clauses.iter().any(|&clause| query.to_uppercase().contains(clause))
}

fn format_as_csv(result: &Value) -> String {
    // Implement CSV formatting here
    // For now, we'll just return the JSON as a string
    result.to_string()
}

async fn create_engine(engine_config: &EngineConfig) -> Result<Box<dyn Engine>, Error> {
    match engine_config.engine.as_str() {
        "openai" => Ok(Box::new(OpenAIEngine::new(engine_config.clone()).await?)),
        "anthropic" => Ok(Box::new(AnthropicEngine::new(engine_config.clone()).await?)),
        "cohere" => Ok(Box::new(CohereEngine::new(engine_config.clone()).await?)),
        "google_gemini" => Ok(Box::new(GoogleGeminiEngine::new(engine_config.clone()).await?)),
        "perplexity" => Ok(Box::new(PerplexityEngine::new(engine_config.clone()).await?)),
        "groq_lpu" => Ok(Box::new(GroqLPUEngine::new(engine_config.clone()).await?)),
        _ => Err(anyhow!("Unsupported engine: {}", engine_config.engine)),
    }
}

async fn create_llm_engine(engine_config: &EngineConfig) -> Result<Box<dyn Engine>, Error> {
    create_engine(engine_config).await
}


