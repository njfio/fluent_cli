mod client;
mod config;

use std::io;
use clap::{Arg, ArgAction, Command};


use tokio;

use log::{debug};

use env_logger;

use tokio::fs::File;

use tokio::io::{AsyncReadExt};

use crate::client::{ handle_response, print_full_width_bar };

use crate::config::{EnvVarGuard, replace_with_env_var};

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
use anyhow::{Context, Result};
use clap_complete_fig::Fig;
use crossterm::style::Stylize;

use tokio::time::Instant;

// use env_logger; // Uncomment this when you are using it to initialize logs
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    colored::control::set_override(true);

    let mut configs = config::load_config().unwrap();
    let configs_clone = configs.clone();

    let  mut command = Command::new("fluent")
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .bin_name("fluent")
        .version("0.3.5")
        .author("Nicholas Ferguson <nick@njf.io>")
        .about("Interacts with FlowiseAI, Langflow, and Webhook workflows")
        .arg(Arg::new("flowname")
            .value_name("FLOWNAME")
            .index(1)
            .help("The flow name to invoke")
            .action(ArgAction::Set)
            .required_unless_present_any([
                "generate-fig-autocomplete",
                "system-prompt-override-inline",
                "system-prompt-override-file",
                "additional-context-file"
            ]))

        .arg(Arg::new("request")
            .index(2)
            .help("The request string to send")
            .action(ArgAction::Set)
            .required_unless_present_any([
                "generate-fig-autocomplete",
                "system-prompt-override-inline",
                "system-prompt-override-file",
                "additional-context-file"
            ]))

        .arg(Arg::new("context")
            .index(3)
            .short('c')  // Assigns a short flag
            .help("Optional context to include with the request")
            .action(ArgAction::Set)
            .required(false))

        .arg(Arg::new("system-prompt-override-inline")
            .long("system-prompt-override-inline")
            .short('i')  // Assigns a short flag
            .help("Overrides the system message with an inline string")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("system-prompt-override-file")
            .long("system-prompt-override-file")
            .short('f')  // Assigns a short flag
            .help("Overrides the system message from a specified file")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("additional-context-file")
            .long("additional-context-file")
            .short('a')  // Assigns a short flag
            .help("Specifies a file from which additional request context is loaded")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("upload-image-path")
            .long("upload-image-path")
            .short('u')  // Assigns a short flag
            .value_name("FILE")
            .help("Sets the input file to use")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("generate-autocomplete")
            .long("generate-autocomplete")
            .help("Generates a bash autocomplete script")
            .action(ArgAction::SetTrue)
            .required(false))

        .arg(Arg::new("generate-fig-autocomplete")
            .long("generate-fig-autocomplete")
            .default_value("false")
            .help("Generates a fig autocomplete script")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("parse-code-output")
            .long("parse-code-output")
            .short('p')  // Assigns a short flag
            .help("Extracts and displays only the code blocks from the response")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("full-output")
            .long("full-output")
            .short('z')  // Assigns a short flag
            .help("Outputs all response data in JSON format")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("markdown-output")
            .long("markdown-output")
            .short('m')  // Assigns a short flag
            .help("Outputs the response to the terminal in stylized markdown. Do not use for pipelines")
            .action(ArgAction::SetTrue))

        .arg(Arg::new("download-media")
            .long("download-media")
            .short('d')  // Assigns a short flag
            .help("Downloads all media files listed in the output to a specified directory")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false)
            .value_name("DIRECTORY"))

        .arg(Arg::new("upsert-no-upload")
            .long("upsert-no-upload")
            .help("Sends a JSON payload to the specified endpoint without uploading files")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("upsert-with-upload")
            .long("upsert-with-upload")
            .value_name("FILE")
            .help("Uploads a file to the specified endpoint")
            .action(ArgAction::Set)  // Use Append if multiple values may be provided
            .required(false))

        .arg(Arg::new("webhook")
            .long("webhook")
            .help("Sends the command payload to the webhook URL specified in config.json")
            .action(ArgAction::SetTrue));

 // Assuming build_cli() properly constructs a clap::Command
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
    let _actual_final_context_clone  = actual_final_context.clone();
    let actual_final_context_clone2  = actual_final_context.clone();

    debug!("Actual Final context: {:?}", actual_final_context);
    let new_question = if let Some(ctx) = actual_final_context {
        format!("\n{}\n{}\n", request, ctx)  // Concatenate request and context
    } else {
        request.to_string()  // Use request as is if no context
    };

    // Decrypt the keys in the flow config
    let mut env_guard = EnvVarGuard::new();
    let env_guard_result = env_guard.decrypt_amber_keys_for_flow(flow).unwrap();
    debug!("EnvGuard result: {:?}", env_guard_result);

    // Within the main function after parsing command-line arguments
    if let Some(files) = matches.get_one::<String>("upsert-with-upload").map(|s| s.as_str()) {
        let file_paths: Vec<&str> = files.split(',').collect();
        debug!("Uploading files: {:?}", file_paths);
        let flow = configs_clone.iter().find(|f| f.name == flowname).expect("Flow not found");
        debug!("Flow: {:?}", flow);

        // Construct the URL using the upsert path
        let api_url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow_clone.upsert_path.unwrap_or_default(), flow.chat_id);
        debug!("API URL: {}", api_url);
        // Call the upload function in the client module
        if let Err(e) = client::upload_files(&api_url, file_paths).await {
            eprintln!("Error uploading files: {}", e.to_string());
        }
    }

    if let Some(json_str) = matches.get_one::<String>("upsert-no-upload").map(|s| s.as_str())  {
        let inline_json: serde_json::Value = serde_json::from_str(json_str).unwrap_or_else(|err| {
            eprintln!("Error parsing inline JSON: {}", err);
            serde_json::json!({})
        });

        let flow = configs_clone.iter().find(|f| f.name == flowname).expect("Flow not found");

        let mut override_config = flow.override_config.clone();
        debug!("Override config before update: {:?}", override_config);
        replace_with_env_var(&mut override_config);
        debug!("Override config after update: {:?}", override_config);

        debug!("Override config: {:?}", override_config);
        // Merge inline JSON with the existing override config
        if let serde_json::Value::Object(ref mut base) = override_config {
            if let serde_json::Value::Object(additional) = inline_json {
                for (key, value) in additional {
                    base.insert(key, value);
                }
            }
        }
        debug!("Merged override config: {:?}", override_config);

        let api_url = format!("{}://{}:{}{}{}", flow.protocol, flow.hostname, flow.port, flow_clone2.upsert_path.unwrap_or_default(), flow.chat_id);
        debug!("API URL: {}", api_url);

        // Use the merged override config as the payload
        if let Err(e) = client::upsert_with_json(&api_url, &flow_clone3, serde_json::json!({"overrideConfig": override_config})).await {
            eprintln!("Error during JSON upsert: {}", e.to_string());
        }
    }

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner()
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
        .expect("Failed to set progress style"));
    spinner.enable_steady_tick(Duration::from_millis(300));


    let engine_type = &flow.engine;
    let start_time = Instant::now();

    print_status(&spinner, flowname, request, actual_final_context_clone2.as_ref().unwrap_or(&new_question).as_str());
    spinner.tick();
    debug!("Preparing Payload");
    let payload = crate::client::prepare_payload(&flow, request, file_path, actual_final_context_clone2, &cli_args, &file_contents_clone ).await?;
    let response = crate::client::send_request(&flow, &payload).await?;
    debug!("Handling Response");

    let duration = start_time.elapsed();
    spinner.finish_with_message(format!(
        "\n{}\n\n\t{}    	{}\n\t{} 	{}\n\t{}	{}\n\n{}\n",
        client::print_full_width_bar("■"),
        "Flow: ".grey().italic(),
        flowname.purple().italic(),
        "Request: ".grey().italic(),
        request.bright_blue().italic(),
        "Duration: ".grey().italic(),
        format!("{:.4}s", duration.as_secs_f32()).green().italic(),  // Apply bright yellow color to duration
        client::print_full_width_bar("-")
    ));

    //spinner.finish_and_clear();
    match engine_type.as_str() {
        "flowise" | "webhook" => {
            // Handle Flowise output
            handle_response(response.as_str(), &matches).await
        },
        "langflow" => {
            // Handle LangFlow output
            client::handle_langflow_response(response.as_str(), &matches).await
        },
        _ => Ok({
            // Handle default outputser);
            serde_json::Error::custom("Unsupported engine type");
        })
    }.expect("TODO: panic message");
    eprint!("\n\n{}\n\n", print_full_width_bar("■"));

    Ok(())
}

