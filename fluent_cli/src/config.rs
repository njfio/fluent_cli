use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;
use std::env;
use std::process::Command;
use std::str;
use std::error::Error;
use log::{debug, info};

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
    pub timeout_ms: Option<u64>,
    pub protocol: String,
}

pub(crate) struct EnvVarGuard {
    keys: Vec<String>,
}

impl EnvVarGuard {
    pub fn new() -> Self {
        EnvVarGuard { keys: Vec::new() }
    }

    pub fn set_env_var_from_amber(&mut self, key: &str, amber_key: &str) -> Result<(), Box<dyn Error>> {
        let output = Command::new("amber")
            .arg("print")
            .output()?;

        debug!("Amber output: {:?}", output);
        if !output.status.success() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to decrypt Amber key")));
        }

        let output_str = str::from_utf8(&output.stdout)?;
        let lines = output_str.lines();

        // Parse the output to find the specific key and its value
        for line in lines {
            if line.contains(amber_key) {
                debug!("Line found: {}", line);
                // Assumes the format: export AMBER_KEY_NAME="value"
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if let Some(value_part) = parts.get(1) {
                    let value = value_part.trim().trim_matches('"');
                    env::set_var(key, value);
                    self.keys.push(key.to_owned());
                    info!("Set environment variable {} with decrypted value.", key);
                    break;
                }
            }
        }
        Ok(())
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            env::remove_var(key);
            info!("Environment variable {} has been unset.", key);
        }
    }
}

pub fn load_config() -> Result<Vec<FlowConfig>, Box<dyn Error>> {
    let config_path = env::var("FLUENT_CLI_CONFIG_PATH")
        .map_err(|_| "FLUENT_CLI_CONFIG_PATH environment variable is not set")?;
    let mut file = File::open(config_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let configs: Vec<FlowConfig> = serde_json::from_str(&contents)?;

    Ok(configs)
}

pub fn generate_json_autocomplete_script() -> String {
    return format!(r#"
# Assuming FLUENT_CLI_CONFIG_PATH points to a JSON file containing configuration
autocomplete_flows() {{
    local current_word="${{COMP_WORDS[COMP_CWORD]}}"
    local flow_names=$(jq -r '.[].name' "$FLUENT_CLI_CONFIG_PATH")
    COMPREPLY=($(compgen -W "${{flow_names}}" -- "$current_word"))
}}
complete -F autocomplete_flows fluent_cli
"#)
}



pub fn generate_bash_autocomplete_script() -> String {

    return format!(r#"
# Assuming FLUENT_CLI_CONFIG_PATH points to a JSON file containing configuration
autocomplete_flows() {{
    local current_word="${{COMP_WORDS[COMP_CWORD]}}"
    local flow_names=$(jq -r '.[].name' "$FLUENT_CLI_CONFIG_PATH")
    COMPREPLY=($(compgen -W "${{flow_names}}" -- "$current_word"))
}}
complete -F autocomplete_flows fluent_cli
"#)

}
