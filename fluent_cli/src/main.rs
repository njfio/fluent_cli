use serde::Deserialize;
use std::fs::File;
use std::io::{self, Read};
use std::env; // Import the env module to access environment variables

#[derive(Debug, Deserialize)]
struct FlowConfig {
    name: String,
    hostname: String,
    port: u16,
    chat_id: String,
    request_path: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    bearer_token: String,
    #[serde(rename = "overrideConfig")]
    override_config: serde_json::Value,
}

fn load_config() -> Result<Vec<FlowConfig>, Box<dyn std::error::Error>> {
    // Fetch the path from the environment variable
    let config_path = env::var("FLUENT_CLI_CONFIG_PATH")
        .map_err(|_| "FLUENT_CLI_CONFIG_PATH environment variable is not set")?;

    let mut file = File::open(config_path.clone())?;
    eprintln!("loading config from: {}", config_path);
    let mut contents = String::new();
    eprintln!("contents: {:?}", contents);
    file.read_to_string(&mut contents)?;
    let flows: Vec<FlowConfig> = serde_json::from_str(&contents)?;
    Ok(flows)
}

use clap::{App, Arg};
use reqwest::Client;
use tokio;
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Fluents CLI")
        .version("0.1.0")
        .author("Nicholas Ferguson <nick@njf.io>")
        .about("Interacts with FlowiseAI workflows")
        .arg(Arg::new("flowname")
            .help("The flow name to invoke")
            .required(true)
            .index(1))
        .arg(Arg::new("request")
            .help("The request string to send")
            .required(true)
            .index(2))
        .get_matches();

    // Load configurations
    let flows = load_config()?;
    let flowname = matches.value_of("flowname").unwrap();
    let request_content = matches.value_of("request").unwrap();

    // Find the corresponding flow configuration
    let flow = flows.iter().find(|f| f.name == flowname).expect("Flow not found");
    let client = Client::new();
    let url = format!("http://{}:{}{}{}", flow.hostname, flow.port, flow.request_path, flow.chat_id);

    // Build the JSON body with the overrideConfig
    let override_config = &flow.override_config;
    let body = json!({
        "question": request_content,
        "overrideConfig": override_config
    });

    // Log the final body to be sent
    eprintln!("Final request body: {}", body);

    // Build the request
    let request_builder = client.post(&url)
        .header("Authorization", format!("Bearer {}", flow.bearer_token))
        .json(&body);

    // Send the request
    let response = request_builder.send().await?;
    let response_body = response.text().await?;
    println!("{}", response_body);

    Ok(())
}