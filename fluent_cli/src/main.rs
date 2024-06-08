mod client;
mod config;
mod openai_agent_client;

use openai_agent_client::{send_openai_request, OpenAIResponse};

use std::io;

use clap::{Arg, ArgAction, ColorChoice, Command};

use log::{debug};

use tokio::fs::File;

use tokio::io::{AsyncReadExt};

use crate::client::{ handle_response, print_full_width_bar };

use crate::config::{EnvVarGuard, FlowConfig, replace_with_env_var};

use colored::*; // Import the colored crate

use serde::de::Error as SerdeError;

use indicatif::{ProgressBar, ProgressStyle};

use std::time::Duration;


use clap_complete::generate;



fn print_status(spinner: &ProgressBar, flowname: &str, request: &str, new_question: &str) {
    spinner.set_message(format!(
        "\n{}\t{}\n{}\t{}\n{}\n{}\n",

        "Flow:  ".purple().italic(),
        flowname.bright_blue().bold(),
        "Request:".purple().italic(),
        request.bright_blue().italic(),
        "Context:".purple().italic(),
        new_question.bright_green(),

    ));
}
use anyhow::{Context, Error, Result};
use clap_complete_fig::Fig;
use crossterm::style::Stylize;

use tokio::time::Instant;

// use env_logger; // Uncomment this when you are using it to initialize logs
use serde_json::{Value};

fn update_value(existing_value: &mut Value, new_value: &str) {
    match existing_value {
        Value::Array(_arr) => {
            // Preserve the array if the existing value is an array
            *existing_value = Value::Array(vec![Value::String(new_value.to_string())]);
        }
        _ => {
            // Default to string replacement
            *existing_value = Value::String(new_value.to_string());
        }
    }
}


use std::collections::HashMap;
use crate::openai_agent_client::Message;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    colored::control::set_override(true);

    let mut configs = config::load_config().unwrap();
    let configs_clone = configs.clone();

    let mut command = Command::new("fluent")
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .bin_name("fluent")
        .version("0.3.5.6")
        .author("Nicholas Ferguson <nick@njf.io>")
        .about("Interacts with FlowiseAI, Langflow, and Webhook workflows")
        .color(ColorChoice::Auto)
        .arg(Arg::new("flowname")
            .value_name("flowname")
            .index(1)
            .help("The flow name to invoke")
            .action(ArgAction::Set)
            .requires("request")
            .required_unless_present_any(["generate-fig-autocomplete"]))
        .arg(Arg::new("request")
            .index(2)
            .help("The request to send the workflow")
            .action(ArgAction::Set)
            .value_parser(clap::builder::NonEmptyStringValueParser::new())
            .requires("flowname")
            .required_unless_present_any(["generate-fig-autocomplete"]))
        .arg(Arg::new("context")
            .help("Optional context provided through <stdin>")
            .action(ArgAction::Set)
            .required(false))
        .arg(Arg::new("additional-context-file")
            .long("additional-context-file")
            .short('a')
            .help("Specifies a file from which additional request context is loaded")
            .action(ArgAction::Set)
            .value_hint(clap::ValueHint::FilePath)
            .required(false))
        .arg(Arg::new("upload-image-path")
            .long("upload-image-path")
            .short('u')
            .value_hint(clap::ValueHint::FilePath)
            .value_name("FilePath")
            .help("Sets the input file to use")
            .action(ArgAction::Set)
            .required(false))
        .arg(Arg::new("upsert-no-upload")
            .long("upsert-no-upload")
            .help("Sends a JSON payload to the specified endpoint without uploading files")
            .action(ArgAction::SetTrue)
            .required(false))
        .arg(Arg::new("upsert-with-upload")
            .long("upsert-with-upload")
            .value_name("FILE")
            .value_hint(clap::ValueHint::FilePath)
            .help("Uploads a file to the specified endpoint")
            .action(ArgAction::Set)
            .required(false))
        .arg(Arg::new("system-prompt-override-inline")
            .long("system-prompt-override-inline")
            .short('i')
            .help("Overrides the system message with an inline string")
            .action(ArgAction::Set)
            .required(false))
        .arg(Arg::new("system-prompt-override-file")
            .long("system-prompt-override-file")
            .short('f')
            .value_hint(clap::ValueHint::FilePath)
            .help("Overrides the system message from a specified file")
            .action(ArgAction::Set)
            .required(false))
        .arg(Arg::new("markdown-output")
            .long("markdown-output")
            .short('m')
            .conflicts_with_all(["full-output", "parse-code-output"])
            .help("Outputs the response to the terminal in stylized markdown. Do not use for pipelines")
            .action(ArgAction::SetTrue))
        .arg(Arg::new("download-media")
            .long("download-media")
            .short('d')
            .help("Downloads all media files listed in the output to a specified directory")
            .action(ArgAction::Set)
            .required(false)
            .value_hint(clap::ValueHint::DirPath)
            .value_name("DIRECTORY"))
        .arg(Arg::new("parse-code-output")
            .long("parse-code-output")
            .short('p')
            .help("Extracts and displays only the code blocks from the response")
            .conflicts_with_all(["full-output", "markdown-output"])
            .action(ArgAction::SetTrue))
        .arg(Arg::new("full-output")
            .long("full-output")
            .short('z')
            .help("Outputs all response data in JSON format")
            .conflicts_with_all(["parse-code-output", "markdown-output"])
            .action(ArgAction::SetTrue))
        .arg(Arg::new("generate-autocomplete")
            .long("generate-autocomplete")
            .help("Generates a bash autocomplete script")
            .action(ArgAction::SetTrue)
            .exclusive(true))
        .arg(Arg::new("generate-fig-autocomplete")
            .long("generate-fig-autocomplete")
            .help("Generates a fig autocomplete script")
            .exclusive(true)
            .action(ArgAction::SetTrue))
        .arg(Arg::new("override")
            .long("override")
            .short('o')
            .value_name("KEY=VALUE")
            .num_args(1..)
            .help("Overrides any entry in the config with the specified key-value pair")
            .action(ArgAction::Append)
            .required(false));

    if std::env::args().any(|arg| arg == "--generate-fig-autocomplete") {
        generate(Fig, &mut command, "fluent", &mut io::stdout());
        return Ok(());
    }

    let matches = &command.get_matches();

    let cli_args = matches.clone();

    let flowname = matches.get_one::<String>("flowname").map(|s| s.as_str()).unwrap();
    let flow = configs.iter_mut().find(|f| f.name == flowname).context("Flow not found")?;
    let flow_clone = flow.clone();
    let flow_clone2 = flow.clone();
    let flow_clone3 = flow.clone();
    let request = matches.get_one::<String>("request").map(|s| s.as_str()).unwrap();

    // Load context from stdin if not provided
    let context = matches.get_one::<String>("context").map(|s| s.as_str());
    let mut additional_context = String::new();
    if context.is_none() && !atty::is(atty::Stream::Stdin) {
        tokio::io::stdin().read_to_string(&mut additional_context).await?;
    }
    debug!("Additional context: {:?}", additional_context);
    let final_context = context.or(if !additional_context.is_empty() { Some(&additional_context) } else { None });
    debug!("Context: {:?}", final_context);

    // Load override value from CLI if specified for system prompt override, file will always win
    let system_prompt_inline = matches.get_one::<String>("system-prompt-override-inline").map(|s| s.as_str());
    let system_prompt_file = matches.get_one::<String>("system-prompt-override-file").map(|s| s.as_str());
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
    if let Some(override_value) = system_message_override {
        if let Some(obj) = flow.override_config.as_object_mut() {
            if obj.contains_key("systemMessage") {
                obj.insert("systemMessage".to_string(), serde_json::Value::String(override_value.to_string()));
            }
            if obj.contains_key("systemMessagePrompt") {
                obj.insert("systemMessagePrompt".to_string(), serde_json::Value::String(override_value.to_string()));
            }
        }
    }

    let file_path = matches.get_one::<String>("upload-image-path").map(|s| s.as_str());
    let _file_path_clone = matches.get_one::<String>("upload-image-path").map(|s| s.as_str());

    // Determine the final context from various sources
    let file_context = matches.get_one::<String>("additional-context-file").map(|s| s.as_str());
    let mut file_contents = String::new();
    if let Some(file_path) = file_context {
        let mut file = File::open(file_path).await?;
        file.read_to_string(&mut file_contents).await?;
    }
    let file_contents_clone = file_contents.clone();

    // Combine file contents with other forms of context if necessary
    let actual_final_context = match (final_context, file_contents.is_empty()) {
        (Some(cli_context), false) => Some(format!("\n{}\n{}\n", cli_context, file_contents)),
        (None, false) => Some(file_contents),
        (Some(cli_context), true) => Some(cli_context.to_string()),
        (None, true) => None,
    };
    let _actual_final_context_clone = actual_final_context.clone();
    let actual_final_context_clone2 = actual_final_context.clone();

    debug!("Actual Final context: {:?}", actual_final_context);
    let new_question = if let Some(ctx) = actual_final_context {
        format!("\n{}\n{}\n", request, ctx) // Concatenate request and context
    } else {
        request.to_string() // Use request as is if no context
    };

    // Decrypt the keys in the flow config
    let mut env_guard = EnvVarGuard::new();
    env_guard.decrypt_amber_keys_for_flow(flow).unwrap();

    // Handle upsert with upload
    if let Some(files) = matches.get_one::<String>("upsert-with-upload").map(|s| s.as_str()) {
        let file_paths: Vec<&str> = files.split(',').collect();
        debug!("Uploading files: {:?}", file_paths);
        let flow = configs_clone.iter().find(|f| f.name == flowname).expect("Flow not found");
        debug!("Flow: {:?}", flow);

        let api_url = format!(
            "{}://{}:{}{}{}",
            flow.protocol, flow.hostname, flow.port, flow_clone.upsert_path.unwrap_or_default(), flow.chat_id
        );
        debug!("API URL: {}", api_url);
        if let Err(e) = client::upload_files(&api_url, file_paths).await {
            eprintln!("Error uploading files: {}", e);
        }
    }



    let mut override_config = flow.override_config.clone();
    let mut tweaks_config = flow.tweaks.clone(); // Assuming flow has a tweaks field

    if let Some(overrides) = matches.get_many::<String>("override") {
        let overrides: HashMap<String, String> = overrides
            .map(|s| parse_key_value_pair(s).unwrap())
            .collect();

        for (key, value) in overrides {
            // Split the key into parts
            let key_parts: Vec<&str> = key.split('.').collect();

            if key_parts.len() == 1 {
                // Update override_config
                if let Some(obj) = override_config.as_object_mut() {
                    if let Some(existing_value) = obj.get_mut(&key) {
                        update_value(existing_value, &value);
                    } else {
                        obj.insert(key.clone(), Value::String(value.clone()));
                    }
                }

                // Update tweaks_config (top-level keys)
                if let Some(tweaks_obj) = tweaks_config.as_object_mut() {
                    for (_, tweak) in tweaks_obj.iter_mut() {
                        if let Some(tweak_obj) = tweak.as_object_mut() {
                            if let Some(existing_value) = tweak_obj.get_mut(&key) {
                                update_value(existing_value, &value);
                            } else {
                                tweak_obj.insert(key.clone(), Value::String(value.clone()));
                            }
                        }
                    }
                }
            } else {
                // Handle nested keys for tweaks_config
                if let Some(tweaks_obj) = tweaks_config.as_object_mut() {
                    if let Some(tweak) = tweaks_obj.get_mut(key_parts[0]) {
                        if let Some(tweak_obj) = tweak.as_object_mut() {
                            if let Some(existing_value) = tweak_obj.get_mut(key_parts[1]) {
                                update_value(existing_value, &value);
                            } else {
                                tweak_obj.insert(key_parts[1].to_string(), Value::String(value.clone()));
                            }
                        }
                    }
                }
            }
        }

        flow.override_config = override_config.clone();
        flow.tweaks = tweaks_config.clone(); // Update the flow's tweaks
    }

    // Handle upsert with no upload
    if matches.get_flag("upsert-no-upload") {
        let flow = configs_clone.iter().find(|f| f.name == flowname).expect("Flow not found");

        debug!("Override config before update: {:?}", override_config);
        replace_with_env_var(&mut override_config);
        debug!("Override config after update: {:?}", override_config);

        debug!("Tweaks config before update: {:?}", tweaks_config);
        replace_with_env_var(&mut tweaks_config);
        debug!("Tweaks config after update: {:?}", tweaks_config);

        let api_url = format!(
            "{}://{}:{}{}{}",
            flow.protocol, flow.hostname, flow.port, flow_clone2.upsert_path.unwrap_or_default(), flow.chat_id
        );
        debug!("API URL: {}", api_url);

        // Use the merged override config and tweaks as the payload
        let payload = serde_json::json!({
            "overrideConfig": override_config,
            "tweaks": tweaks_config
        });

        if let Err(e) = client::upsert_with_json(&api_url, &flow_clone3, payload).await {
            eprintln!("Error during JSON upsert: {}", e);
        }
    }


    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&[
                "",
                "⫸ ",
                "⫸⫸ ",
                "⫸⫸⫸ ",
                "⫸⫸⫸⫸ ",
                "💛⫸⫸⫸⫸ ",
                "⫸💛⫸⫸⫸ ",
                "⫸⫸💛⫸⫸ ",
                "⫸⫸⫸💛⫸ ",
                "⫸⫸⫸⫸💛 ",
                "⫸⫸⫸💛⫷ ",
                "⫸⫸💛⫷⫷ ",
                "⫸💛⫷⫷⫷ ",
                "💛⫷⫷⫷⫷ ",
                "⫷⫷⫷⫷ ",
                "⫷⫷⫷ ",
                "⫷⫷ ",
                "⫷ ",
                " ",
            ])
            .template("{spinner:.yellow}{msg}{spinner:.yellow}")
            .expect("Failed to set progress style"),
    );
    spinner.enable_steady_tick(Duration::from_millis(300));

    let engine_type = &flow.engine;
    let start_time = Instant::now();

    print_status(&spinner, flowname, request, actual_final_context_clone2.as_ref().unwrap_or(&new_question).as_str());
    spinner.tick();
    let prompt = format!("{} {}", request, &actual_final_context_clone2.as_ref().unwrap_or(&new_question));

    debug!("Handling Response");

    let output = match flow.engine.as_str() {
        "flowise" | "webhook" => {

            // Handle Flowise output
            let payload = crate::client::prepare_payload(flow, request, file_path, actual_final_context_clone2, &cli_args, &file_contents_clone).await?;
            let response = crate::client::send_request(flow, &payload).await?;
            let duration = start_time.elapsed(); // Capture the duration after the operation completes

            spinner.finish_with_message(format!(
                "\n{}\n\n\t{}    	{}\n\t{} 	{}\n\t{}	{}\n\n{}\n",
                client::print_full_width_bar("■"),
                "Flow: ".grey().italic(),
                flowname.purple().italic(),
                "Request: ".grey().italic(),
                request.bright_blue().italic(),
                "Duration: ".grey().italic(),
                format!("{:.4}s", duration.as_secs_f32()).green().italic(), // Apply bright yellow color to duration
                client::print_full_width_bar("-")
            ));
            handle_response(response.as_str(), matches).await
        }
        "langflow" => {

            let payload = crate::client::prepare_payload(flow, request, file_path, actual_final_context_clone2, &cli_args, &file_contents_clone).await?;
            let response = crate::client::send_request(flow, &payload).await?;
            let duration = start_time.elapsed(); // Capture the duration after the operation completes

            spinner.finish_with_message(format!(
                "\n{}\n\n\t{}    	{}\n\t{} 	{}\n\t{}	{}\n\n{}\n",
                client::print_full_width_bar("■"),
                "Flow: ".grey().italic(),
                flowname.purple().italic(),
                "Request: ".grey().italic(),
                request.bright_blue().italic(),
                "Duration: ".grey().italic(),
                format!("{:.4}s", duration.as_secs_f32()).green().italic(), // Apply bright yellow color to duration
                client::print_full_width_bar("-")
            ));
            // Handle LangFlow output
            client::handle_langflow_response(response.as_str(), matches).await

        }
        "openai" => {

            // Handle OpenAI output

            match openai_agent_client::handle_openai_agent(&prompt, &flow, matches).await {

                Ok(response) => {
                    let duration = start_time.elapsed(); // Capture the duration after the operation completes

                    spinner.finish_with_message(format!(
                        "\n{}\n\n\t{}    	{}\n\t{} 	{}\n\t{}	{}\n\n{}\n",
                        client::print_full_width_bar("■"),
                        "Flow: ".grey().italic(),
                        flowname.purple().italic(),
                        "Request: ".grey().italic(),
                        request.bright_blue().italic(),
                        "Duration: ".grey().italic(),
                        format!("{:.4}s", duration.as_secs_f32()).green().italic(), // Apply bright yellow color to duration
                        client::print_full_width_bar("-")
                    ));
                    client::handle_openai_response(&response, &matches).await?
                },
                Err(e) => eprintln!("Error handling OpenAI response: {}", e),
            };
            Ok(())
        }
        _ => {
            // Handle default output);
            return Err(Error::from(serde_json::Error::custom("Unsupported engine type")));
        }
    }
        .expect("TODO: panic message");
    eprint!("\n\n{}\n\n", print_full_width_bar("■"));

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


const MAX_ITERATIONS: usize = 10;

