use owo_colors::OwoColorize;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpinnerConfig {
    pub frames: String,
    pub interval: u64,
    pub success_symbol: String,
    pub failure_symbol: String,
    pub success_symbol_colored: String,
    pub failure_symbol_colored: String,
}

impl Default for SpinnerConfig {
    fn default() -> Self {
        let success_symbol = "\n✓ Success ✓".to_string();
        let failure_symbol = "\n✗ Failure ✗".to_string();

        SpinnerConfig {
            frames: "-\\|/ ".to_string(),
            interval: 100,
            success_symbol: success_symbol.clone(),
            failure_symbol: failure_symbol.clone(),
            success_symbol_colored: success_symbol.green().to_string(),
            failure_symbol_colored: failure_symbol.red().to_string(),
        }
    }
}
