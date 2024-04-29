use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::File;
use std::io::Read;
use std::env;
use std::process::Command;
use std::str;
use std::error::Error;
use log::{debug, info};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FlowConfig {
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub chat_id: String,
    pub request_path: String,
    pub upsert_path: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub bearer_token: String,
    #[serde(rename = "overrideConfig")]
    pub override_config: Value,
    pub timeout_ms: Option<u64>,
    pub protocol: String,
    pub webhook_url: Option<String>,
    pub webhook_headers: Option<Value>,
    pub engine: String,
}



// Helper function to replace config strings starting with "AMBER_" with their env values
pub(crate) fn replace_with_env_var(value: &mut Value) {
    match value {
        Value::String(s) if s.starts_with("AMBER_") => {
            let env_key = &s[6..]; // Skip the "AMBER_" prefix to fetch the correct env var
            debug!("Attempting to replace: {}", s);
            debug!("Looking up environment variable: {}", env_key);
            match env::var(env_key) {
                Ok(env_value) => {
                    debug!("Environment value found: {}", env_value);
                    *s = env_value; // Successfully replace the string with the environment variable value.
                },
                Err(e) => {
                    debug!("Failed to find environment variable '{}': {}", env_key, e);
                    // Optionally, handle the error by logging or defaulting
                }
            }
        },
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                debug!("Inspecting object key: {}", k);
                replace_with_env_var(v); // Recursively replace values in map
            }
        },
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                replace_with_env_var(item); // Recursively replace values in array
            }
        },
        _ => {} // No action required for other types
    }
}



pub(crate) struct EnvVarGuard {
    keys: Vec<String>,
}

impl EnvVarGuard {
    pub fn new() -> Self {
        EnvVarGuard { keys: Vec::new() }
    }

    pub fn decrypt_amber_keys_for_flow(&mut self, flow: &mut FlowConfig) -> Result<(), Box<dyn Error>> {
        // Check each field in FlowConfig if it starts with "AMBER_"
        if flow.bearer_token.starts_with("AMBER_") {
            debug!("Decrypting bearer_token: {}", &flow.bearer_token);
            self.set_env_var_from_amber("bearer_token", &flow.bearer_token[6..])?;
        }
        if flow.session_id.starts_with("AMBER_") {
            debug!("Decrypting session_id: {}", &flow.session_id);
            self.set_env_var_from_amber("session_id", &flow.session_id[6..])?;
        }
        // Assume overrideConfig might also have Amber encrypted keys
        self.decrypt_amber_keys_in_value(&mut flow.override_config)?;
        debug!("Decrypted keys: {:?}", self.keys);
        Ok(())
    }

    fn decrypt_amber_keys_in_value(&mut self, value: &mut Value) -> Result<(), Box<dyn Error>> {
        debug!("Decrypting value: {:?}", value);
        if let Value::Object(obj) = value {
            debug!("Decrypting object: {:?}", obj);
            for (key, val) in obj.iter_mut() {
                debug!("Decrypting key: {}, value: {:?}", key, val);
                if let Some(str_val) = val.as_str() {
                    debug!("Decrypting str_val: {}", str_val);
                    if str_val.starts_with("AMBER_") {
                        debug!("Decrypting str_val: {}", str_val);
                        self.set_env_var_from_amber(key, &str_val[6..])?;
                        debug!("Decrypted key: {}, value: {:?}", key, val);
                    }
                } else {
                    self.decrypt_amber_keys_in_value(val)?;
                    debug!("Decrypted value: {:?}", val);
                }
            }
        }
        Ok(())
    }

    fn set_env_var_from_amber(&mut self, env_key: &str, amber_key: &str) -> Result<(), Box<dyn Error>> {
        let output = Command::new("amber")
            .args(&["print"])
            .output()?;
        debug!("Output of 'amber print {}': {}", amber_key, str::from_utf8(&output.stdout)?);

        if !output.status.success() {
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to decrypt Amber key")));
        }

        // This assumes the `amber print` outputs in the format: export AMBER_KEY_NAME="value"
        // This will find the line containing the amber_key and extract everything after the first '='
        let value = str::from_utf8(&output.stdout)?
            .lines()
            .find(|line| line.contains(amber_key))
            .ok_or("Amber key not found in the output")?
            .split_once('=')
            .ok_or("Invalid format of Amber output")?
            .1 // Take the part after the '='
            .trim() // Trim whitespace
            .trim_matches('"'); // Trim surrounding quotation marks if any

        // Use the amber_key as the environment variable name directly
        env::set_var(amber_key, value);  // Here, use amber_key instead of env_key
        debug!("Set environment variable {} with value {}", amber_key, value);
        self.keys.push(amber_key.to_owned());  // Store amber_key to track what has been set
        info!("Set environment variable {} with decrypted value.", amber_key);
        debug!("Keys: {:?}", self.keys);
        debug!("Environment variables: {:?}", env::vars());

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


