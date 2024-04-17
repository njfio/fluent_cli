mod config;
mod client;

use clap::{App, Arg};
use tokio;

use log::{info, warn, error, debug};
use env_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Starting Fluent CLI");
    let matches = App::new("Fluents CLI")
        .version("0.1.0")
        .author("Nicholas Ferguson <nick@njf.io>")
        .about("Interacts with FlowiseAI workflows")
        .arg(Arg::new("flowname").help("The flow name to invoke").required(true).index(1))
        .arg(Arg::new("request").help("The request string to send").required(true).index(2))
        .get_matches();

    let flows = config::load_config()?;
    debug!("Loaded flows: {:?}", flows);
    let flowname = matches.value_of("flowname").unwrap();
    info!("Invoking flow: {}", flowname);
    let request_content = matches.value_of("request").unwrap();
    debug!("Request: {}", request_content);

    let flow = flows.iter().find(|f| f.name == flowname).expect("Flow not found");
    debug!("Found flow: {:?}", flow);
    match client::send_request(flow, request_content).await {
        Ok(response_body) => println!("{}", response_body),
        Err(e) => eprintln!("Failed to send request: {}", e),
    }

    Ok(())
}
