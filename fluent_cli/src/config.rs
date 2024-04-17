use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{env, fs::File, io::Read, error::Error};
use log::debug;

#[derive(Debug, Deserialize)]
pub struct FlowConfig {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub chat_id: String,
    pub request_path: String,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub bearer_token: String,
    #[serde(rename = "overrideConfig")]
    pub override_config: Value,
    pub timeout_ms: Option<u64>
}

pub fn load_config() -> Result<Vec<FlowConfig>, Box<dyn Error>> {
    debug!("Loading config");
    debug!("Config path: {:?}", env::var("FLUENT_CLI_CONFIG_PATH"));
    let config_path = env::var("FLUENT_CLI_CONFIG_PATH")
        .map_err(|_| "FLUENT_CLI_CONFIG_PATH environment variable is not set")?;
    let mut file = File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    serde_json::from_str(&contents).map_err(Into::into)
}
