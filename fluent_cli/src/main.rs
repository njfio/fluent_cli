use fluent_cli::config;
use fluent_cli::api;
use api::make_api_call;

// src/main.rs


fn main() {
    match config::loader::Config::load_config() {
        Ok(config) => {
            println!("System configuration loaded: {:?}", config.system);
            println!("Flow configuration loaded: {:?}", config.flow);
        },
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    }
}
