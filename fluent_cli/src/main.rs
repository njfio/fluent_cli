mod config;
mod client;

use clap::{App, Arg, Command};
use tokio;

use log::{info, warn, error, debug};
use env_logger;
use tokio::io::{self, AsyncReadExt};

use crate::config::generate_bash_autocomplete_script;

// use env_logger; // Uncomment this when you are using it to initialize logs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let matches = Command::new("Fluent CLI")
        .version("0.1.0")
        .author("Your Name <your.email@example.com>")
        .about("Interacts with FlowiseAI workflows")
        .arg(Arg::new("flowname")
            .help("The flow name to invoke")
            .takes_value(true)
            .required(true))
        .arg(Arg::new("request")
            .help("The request string to send")
            .takes_value(true)
            .required(true))
        .arg(Arg::new("context")
            .help("Optional context to include with the request")
            .takes_value(true)
            .required(false))
        .arg(Arg::new("generate-autocomplete")
            .long("generate-autocomplete")
            .help("Generates a bash autocomplete script")
            .takes_value(false))
        .after_help("Send the contents of stdin as a request or provide an additional context")
        .get_matches();

    if matches.contains_id("generate-autocomplete") {
        println!("{}", generate_bash_autocomplete_script()?);
        return Ok(());
    }

    let question = matches.value_of("request").unwrap(); // Get the question directly from command line arguments
    debug!("Question: {}", question);
    let flowname = matches.value_of("flowname").unwrap();
    debug!("flowname: {}", flowname);
    let request = matches.value_of("request").unwrap();
    debug!("Request: {}", request);
    let context = matches.value_of("context");
    debug!("context: {:?}", context);

    // Optionally read from stdin if no context is provided via command line
    let mut additional_context = String::new();
    debug!("Additional context: {:?}", context);
    if context.is_none() && !atty::is(atty::Stream::Stdin) {
        tokio::io::stdin().read_to_string(&mut additional_context).await?;
        debug!("Additional context: {:?}", additional_context);
    }
    let final_context = context.or(if !additional_context.is_empty() { Some(&additional_context) } else { None });
    debug!("Final context: {:?}", final_context);
    debug!("Question: {}", question);

    let payload = client::build_request_payload(question, final_context);
    let payload_string = serde_json::to_string(&payload)?;
    debug!("Payload before sending: {:?}", payload);
    // Assume function `send_request` sends the data
    let flows = config::load_config()?;
    debug!("Flows: {:?}", flows);
    let flow = flows.iter().find(|f| f.name == flowname).expect("Flow not found");
    debug!("Flow: {:?}", flow);
    match client::send_request(&flow, &payload_string).await {
        Ok(response_body) => println!("{}", response_body),
        Err(e) => eprintln!("Failed to send request: {}", e),
    }

    Ok(())
}
