mod config;
mod client;

use ::config::Value;
use clap::{App, Arg, Command};
use tokio;

use log::{info, warn, error, debug};
use env_logger;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};
use crate::client::handle_response;

use crate::config::{EnvVarGuard, generate_bash_autocomplete_script};
use anyhow::Result;


// use env_logger; // Uncomment this when you are using it to initialize logs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mut configs = config::load_config()?;

    let matches = Command::new("Fluent CLI")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("Interacts with FlowiseAI workflows")
        .arg(Arg::new("flowname")
            .help("The flow name to invoke")
            .takes_value(true)
            .required_unless_present_any([
                "generate-autocomplete",
                "system-prompt-override-inline",
                "system-prompt-override-file",
                "additional-context-file"
            ]))
        .arg(Arg::new("request")
            .help("The request string to send")
            .takes_value(true)
            .required_unless_present_any([
                "generate-autocomplete",
                "system-prompt-override-inline",
                "system-prompt-override-file",
                "additional-context-file"
            ]))
        .arg(Arg::new("context")
            .short('c')  // Assigns a short flag
            .help("Optional context to include with the request")
            .takes_value(true))
        .arg(Arg::new("system-prompt-override-inline")
            .long("system-prompt-override-inline")
            .short('i')  // Assigns a short flag
            .help("Overrides the system message with an inline string")
            .takes_value(true))
        .arg(Arg::new("system-prompt-override-file")
            .long("system-prompt-override-file")
            .short('f')  // Assigns a short flag
            .help("Overrides the system message from a specified file")
            .takes_value(true))
        .arg(Arg::new("additional-context-file")
            .long("additional-context-file")
            .short('a')  // Assigns a short flag
            .help("Specifies a file from which additional request context is loaded")
            .takes_value(true))
        .arg(Arg::new("upload-image-path")
            .long("upload-image-path")
            .short('u')  // Assigns a short flag
            .value_name("FILE")
            .help("Sets the input file to use")
            .takes_value(true))
        .arg(Arg::new("generate-bash-autocomplete")
            .long("generate-autocomplete")
            .short('g')  // Assigns a short flag
            .help("Generates a bash autocomplete script")
            .takes_value(false))
        .arg(Arg::new("parse-code-output")
            .long("parse-code-output")
            .short('p')  // Assigns a short flag
            .help("Extracts and displays only the code blocks from the response")
            .takes_value(false))
        .arg(Arg::new("full-output")
            .long("full-output")
            .short('z')  // Assigns a short flag
            .help("Outputs all response data in JSON format")
            .takes_value(false))
        .arg(Arg::new("markdown-output")
            .long("markdown-output")
            .short('m')  // Assigns a short flag
            .help("Outputs the response to the terminal in stylized markdown. Do not use for pipelines")
            .takes_value(false))
        .arg(Arg::new("download-media")
            .long("download-media")
            .short('d')  // Assigns a short flag
            .help("Downloads all media files listed in the output to a specified directory")
            .takes_value(true)
            .value_name("DIRECTORY"))
        .get_matches();


    if matches.contains_id("generate-bash-autocomplete") {
        println!("{}", generate_bash_autocomplete_script());
        return Ok(());
    }

    let flowname = matches.value_of("flowname").unwrap();
    let flow = configs.iter_mut().find(|f| f.name == flowname).expect("Flow not found");

    let request = matches.value_of("request").unwrap();

    // Load context from stdin if not provided
    let context = matches.value_of("context");
    let mut additional_context = String::new();
    if context.is_none() && !atty::is(atty::Stream::Stdin) {
        tokio::io::stdin().read_to_string(&mut additional_context).await?;
    }
    debug!("Additional context: {:?}", additional_context);
    let final_context = context.or(if !additional_context.is_empty() { Some(&additional_context) } else { None });
    debug!("Context: {:?}", final_context);

    // Load override value from CLI if specified for system prompt override, file will always win
    let system_prompt_inline = matches.value_of("system-prompt-override-inline");
    let system_prompt_file = matches.value_of("system-prompt-override-file");
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
    let file_path = matches.value_of("upload-image-path");



    // Determine the final context from various sources
    let file_context = matches.value_of("additional-context-file");
    let mut file_contents = String::new();
    if let Some(file_path) = file_context {
        let mut file = File::open(file_path).await?;
        file.read_to_string(&mut file_contents).await?;
    }


    // Combine file contents with other forms of context if necessary
    let actual_final_context = match (final_context, file_contents.is_empty()) {
        (Some(cli_context), false) => Some(format!("{} {}", cli_context, file_contents)),
        (None, false) => Some(file_contents),
        (Some(cli_context), true) => Some(cli_context.to_string()),
        (None, true) => None,
    };
    let actual_final_context_clone  = actual_final_context.clone();

    debug!("Actual Final context: {:?}", actual_final_context);
    let new_question = if let Some(ctx) = actual_final_context {
        format!("{} {}", request, ctx)  // Concatenate request and context
    } else {
        request.to_string()  // Use request as is if no context
    };


    // Decrypt the keys in the flow config
    let mut env_guard = EnvVarGuard::new();
    let env_guard_result = env_guard.decrypt_amber_keys_for_flow(flow)?;
    debug!("EnvGuard result: {:?}", env_guard_result);

    let payload = crate::client::prepare_payload(&flow, request, file_path, actual_final_context_clone ).await?;
    let response = crate::client::send_request(&flow, &payload).await?;
    debug!("Handling Response");
    handle_response(response.as_str(), &matches).await?;
    Ok(())
}

