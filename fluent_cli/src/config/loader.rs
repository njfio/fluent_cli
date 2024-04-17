// src/config/loader.rs
use std::fs::File;
use std::io::{self, Read};
use std::env;
use serde::{Deserialize, Serialize};
use serde_json::Error as SerdeError;

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemConfig {
    pub default_flow: String,
    pub api_key: Option<String>,
    pub amber_config_path: String,
    pub media_download_folder: String,
    pub session_id_env_var: String,
    pub configuration_file_path_env_var: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlowConfig {
    pub flows: Vec<Flow>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Flow {
    pub name: String,
    pub host: String,
    pub chat_id: String,
    pub api_key: String,
    pub override_config: OverrideConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OverrideConfig {
    pub priority: String,
    pub response_mode: String,
}

#[derive(Debug)]
pub struct Config {
    pub system: SystemConfig,
    pub flow: FlowConfig,
}

impl Config {
    pub fn load_config() -> Result<Self, io::Error> {
        let system_config = Config::load_system_config()?;
        let flow_config = Config::load_flow_config()?;

        Ok(Config {
            system: system_config,
            flow: flow_config,
        })
    }

    fn load_system_config() -> Result<SystemConfig, io::Error> {
        let system_config_path = env::var("FLUENT_CLI_SYSTEM_CONFIG_PATH")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "Environment variable FLUENT_CLI_SYSTEM_CONFIG_PATH is not set"))?;
        let mut file = File::open(system_config_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    fn load_flow_config() -> Result<FlowConfig, io::Error> {
        let flow_config_path = env::var("FLUENT_CLI_FLOW_CONFIG_PATH")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "Environment variable FLUENT_CLI_FLOW_CONFIG_PATH is not set"))?;
        let mut file = File::open(flow_config_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
