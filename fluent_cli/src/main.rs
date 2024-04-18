mod config;
mod client;

use clap::{App, Arg, Command};
use tokio;

use log::{info, warn, error, debug};
use env_logger;
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
        .arg(Arg::new("flowname").help("The flow name to invoke").takes_value(true).required_unless_present("generate-autocomplete"))
        .arg(Arg::new("request").help("The request string to send").takes_value(true).required_unless_present("generate-autocomplete"))
        .arg(Arg::new("context").help("Optional context to include with the request").takes_value(true).required(false))
        .arg(Arg::new("generate-bash-autocomplete").long("generate-autocomplete").help("Generates a bash autocomplete script").takes_value(false))
        .after_help("Send the contents of stdin as a request or provide an additional context")
        .get_matches();

    if matches.contains_id("generate-bash-autocomplete") {
        println!("{}", generate_bash_autocomplete_script());
        return Ok(());
    }

    let flowname = matches.value_of("flowname").unwrap();
    let request = matches.value_of("request").unwrap();
    let context = matches.value_of("context");

    let mut additional_context = String::new();
    if context.is_none() && !atty::is(atty::Stream::Stdin) {
        tokio::io::stdin().read_to_string(&mut additional_context).await?;
    }
    let final_context = context.or(if !additional_context.is_empty() { Some(&additional_context) } else { None });

    let payload = client::build_request_payload(request, final_context);

    let flow = configs.iter_mut().find(|f| f.name == flowname).expect("Flow not found");
    let mut env_guard = EnvVarGuard::new();
    let env_guard_result = env_guard.decrypt_amber_keys_for_flow(flow)?;
    debug!("EnvGuard result: {:?}", env_guard_result);


    match client::send_request(flow, &serde_json::to_string(&payload)?).await {
        Ok(response_body) => println!("{}", response_body),
        Err(e) => eprintln!("Failed to send request: {}", e),
    }

    Ok(())
}

