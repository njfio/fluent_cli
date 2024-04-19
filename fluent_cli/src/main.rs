mod config;
mod client;

use ::config::Value;
use clap::{App, Arg, Command};
use tokio;

use log::{info, warn, error, debug};
use env_logger;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt};

use crate::config::{EnvVarGuard, generate_bash_autocomplete_script};

// use env_logger; // Uncomment this when you are using it to initialize logs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let mut configs = config::load_config()?;

    let matches = Command::new("Fluent CLI")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("Interacts with FlowiseAI workflows")
        .arg(Arg::new("flowname").help("The flow name to invoke").takes_value(true).required_unless_present_any(["generate-autocomplete", "System-Prompt-Override-Inline", "System-Prompt-Override-File"]))
        .arg(Arg::new("request").help("The request string to send").takes_value(true).required_unless_present_any(["generate-autocomplete", "System-Prompt-Override-Inline", "System-Prompt-Override-File"]))
        .arg(Arg::new("context").help("Optional context to include with the request").takes_value(true))
        .arg(Arg::new("System-Prompt-Override-Inline").long("System-Prompt-Override-Inline").help("Overrides the system message with an inline string").takes_value(true))
        .arg(Arg::new("System-Prompt-Override-File").long("System-Prompt-Override-File").help("Overrides the system message from a specified file").takes_value(true))
        .arg(Arg::new("generate-bash-autocomplete").long("generate-autocomplete").help("Generates a bash autocomplete script").takes_value(false))
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
    let final_context = context.or(if !additional_context.is_empty() { Some(&additional_context) } else { None });

    // Load override value from CLI if specified for system prompt override, file will always win
    let system_prompt_inline = matches.value_of("System-Prompt-Override-Inline");
    let system_prompt_file = matches.value_of("System-Prompt-Override-File");

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
    if let Some(override_message) = system_message_override {
        if let Some(obj) = flow.override_config.as_object_mut() {
            obj.insert("systemMessage".to_string(), serde_json::Value::String(override_message)); // Correct use of Value::String
        }
    }

    // Build the request payload
    let payload = client::build_request_payload(request, final_context);

    // Decrypt the keys in the flow config
    let mut env_guard = EnvVarGuard::new();
    let env_guard_result = env_guard.decrypt_amber_keys_for_flow(flow)?;
    debug!("EnvGuard result: {:?}", env_guard_result);


    match client::send_request(flow, &serde_json::to_string(&payload)?).await {
        Ok(response_body) => {
            if let Err(e) = client::handle_response(&response_body) {
                eprintln!("Error processing response: {}", e);
            }
        },
        Err(e) => eprintln!("Failed to send request: {}", e),
    }
    Ok(())
}

