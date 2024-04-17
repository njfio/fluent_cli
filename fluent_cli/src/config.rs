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
    pub timeout_ms: Option<u64>,
    pub protocol: String,
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


// Function to get only names of the flows for autocomplete purposes
pub fn get_flow_names() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let flows = load_config()?;
    debug!("Flows: {:?}", flows);
    Ok(flows.iter().map(|flow| flow.name.clone()).collect())
}

pub fn generate_bash_autocomplete_script() -> Result<String, Box<dyn std::error::Error>> {
    let names = get_flow_names()?;
    let mut script = String::from(r#"
_fluent_cli_autocomplete() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    local opts=""
    "#);
    script.push_str(&format!("opts=\"{}\"", names.join(" ")));
    script.push_str(r#"
    COMPREPLY=($(compgen -W "${opts}" -- ${cur}))
    return 0
}
complete -F _fluent_cli_autocomplete fluent_cli
"#);
    Ok(script)
}
