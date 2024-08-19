pub mod args;
use crate::v2::args::{Commands, FluentArgs};
use anyhow::Result;
use anyhow::{anyhow, Error};
use args::RequestSharedArgs;
use clap::Parser;
use fluent_core::config::EngineConfig;
use fluent_core::spinner_configuration::SpinnerConfig;
use fluent_core::traits::Engine;
use fluent_core::types::Request;
use fluent_engines::anthropic::AnthropicEngine;
use fluent_engines::openai::OpenAIEngine;
use fluent_engines::pipeline_executor::{FileStateStore, Pipeline, PipelineExecutor, StateStore};
use fluent_sdk::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use log::debug;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::Instant;

pub async fn run() -> Result<()> {
    eprintln!("Starting CLI v2");
    let args = FluentArgs::parse();

    match args.command {
        Commands::Pipeline(pipeline) => run_pipeline(pipeline).await,
        Commands::OpenAiChat { shared, request } => run_request(shared, request.as_request()).await,
    }
}
fn create_progress_bar(engine: &str) -> ProgressBar {
    let spinner_config = SpinnerConfig::default();
    let pb = ProgressBar::new_spinner();
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars(&spinner_config.frames)
        .template("{spinner:.green} {msg}")
        .unwrap();
    pb.set_style(spinner_style);
    pb.set_message(format!("Processing {:?} request...", engine));
    pb.enable_steady_tick(Duration::from_millis(spinner_config.interval));
    pb.set_length(100);
    pb
}
async fn run_request(shared: RequestSharedArgs, request: FluentRequest) -> Result<()> {
    let _fluent_response = request.run().await?;
    let _start_time = Instant::now();
    let _pb = if let Some(engine) = request.engine.map(|e| e.to_string()) {
        create_progress_bar(&engine)
    } else {
        return Err(anyhow!("Engine is required"));
    };
    if let Some(_generate_cypher) = shared.generate_cypher {}
    Ok(())
}

async fn run_pipeline(pipeline: args::Pipeline) -> Result<(), Error> {
    let pipeline_file = pipeline.file;
    let input = pipeline.input;
    let force_fresh = pipeline.force_fresh;
    let run_id = pipeline.run_id;
    let json_output = pipeline.json_output;
    let pipeline: Pipeline = serde_yaml::from_str(&std::fs::read_to_string(pipeline_file)?)?;
    let state_store_dir = PathBuf::from("./pipeline_states");
    tokio::fs::create_dir_all(&state_store_dir).await?;
    let state_store = FileStateStore {
        directory: state_store_dir,
    };
    let executor = PipelineExecutor::new(state_store.clone(), json_output);
    executor
        .execute(&pipeline, &input, force_fresh, run_id.clone())
        .await?;
    if json_output {
        // Read the state file and print its contents to stdout
        let state_key = format!(
            "{}-{}",
            pipeline.name,
            run_id.unwrap_or_else(|| "unknown".to_string())
        );
        if let Some(state) = state_store.load_state(&state_key).await? {
            println!("{}", serde_json::to_string_pretty(&state)?);
        } else {
            eprintln!("No state file found for the given run ID.");
            std::process::exit(1);
        }
    };
    Ok(())
}

pub async fn generate_cypher_query(query: &str, config: &EngineConfig) -> Result<String> {
    // Use the configured LLM to generate a Cypher query
    let llm_request = Request {
        flowname: "cypher_generation".to_string(),
        payload: format!(
            "Generate a Cypher query for Neo4j based on this request: {}",
            query
        ),
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
