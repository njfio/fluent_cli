use super::{ToolExecutor, ToolExecutionConfig, validation};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Rust compiler tool executor that provides Cargo integration
pub struct RustCompilerExecutor {
    config: ToolExecutionConfig,
    project_root: PathBuf,
}

impl RustCompilerExecutor {
    /// Create a new Rust compiler executor with the given configuration and project root
    pub fn new(config: ToolExecutionConfig, project_root: PathBuf) -> Self {
        Self {
            config,
            project_root,
        }
    }

    /// Create a Rust compiler executor with default configuration
    pub fn with_defaults(project_root: PathBuf) -> Self {
        let mut config = ToolExecutionConfig::default();
        config.allowed_commands = vec![
            "cargo build".to_string(),
            "cargo test".to_string(),
            "cargo check".to_string(),
            "cargo clippy".to_string(),
            "cargo fmt".to_string(),
            "cargo clean".to_string(),
            "cargo doc".to_string(),
            "rustc --version".to_string(),
            "cargo --version".to_string(),
        ];
        config.timeout_seconds = 300; // 5 minutes for compilation
        
        Self::new(config, project_root)
    }

    /// Execute a cargo command safely with timeout and output limits
    async fn execute_cargo_command(&self, subcommand: &str, args: &[String]) -> Result<CargoResult> {
        let start_time = std::time::Instant::now();

        let mut cmd = Command::new("cargo");
        cmd.arg(subcommand)
            .args(args)
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let timeout_duration = Duration::from_secs(self.config.timeout_seconds);
        
        let output = timeout(timeout_duration, cmd.output()).await
            .map_err(|_| anyhow!("Cargo command timed out after {} seconds", self.config.timeout_seconds))?
            .map_err(|e| anyhow!("Failed to execute cargo command: {}", e))?;

        let execution_time = start_time.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let sanitized_stdout = validation::sanitize_output(&stdout, self.config.max_output_size);
        let sanitized_stderr = validation::sanitize_output(&stderr, self.config.max_output_size);

        Ok(CargoResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: sanitized_stdout,
            stderr: sanitized_stderr,
            execution_time_ms: execution_time.as_millis() as u64,
            success: output.status.success(),
            subcommand: subcommand.to_string(),
        })
    }

    /// Check if the project root contains a Cargo.toml file
    async fn validate_cargo_project(&self) -> Result<()> {
        let cargo_toml = self.project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Err(anyhow!(
                "No Cargo.toml found in project root: {}",
                self.project_root.display()
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl ToolExecutor for RustCompilerExecutor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        // Validate that we're in a Cargo project
        self.validate_cargo_project().await?;

        match tool_name {
            "cargo_build" => {
                let release = parameters.get("release")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                
                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if release {
                    args.push("--release".to_string());
                }
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }

                let result = self.execute_cargo_command("build", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_test" => {
                let test_name = parameters.get("test_name")
                    .and_then(|v| v.as_str());
                
                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let lib_only = parameters.get("lib_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
                if lib_only {
                    args.push("--lib".to_string());
                }
                if let Some(test) = test_name {
                    args.push(test.to_string());
                }

                let result = self.execute_cargo_command("test", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_check" => {
                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }

                let result = self.execute_cargo_command("check", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_clippy" => {
                let fix = parameters.get("fix")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
                if fix {
                    args.push("--fix".to_string());
                }

                let result = self.execute_cargo_command("clippy", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_fmt" => {
                let check = parameters.get("check")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
                if check {
                    args.push("--check".to_string());
                }

                let result = self.execute_cargo_command("fmt", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_clean" => {
                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }

                let result = self.execute_cargo_command("clean", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "cargo_doc" => {
                let open = parameters.get("open")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let package = parameters.get("package")
                    .and_then(|v| v.as_str());

                let mut args = Vec::new();
                if let Some(pkg) = package {
                    args.push("--package".to_string());
                    args.push(pkg.to_string());
                }
                if open {
                    args.push("--open".to_string());
                }

                let result = self.execute_cargo_command("doc", &args).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "get_cargo_info" => {
                let cargo_version = self.execute_cargo_command("--version", &[]).await?;
                
                let mut rustc_cmd = Command::new("rustc");
                rustc_cmd.arg("--version")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped());

                let rustc_output = rustc_cmd.output().await
                    .map_err(|e| anyhow!("Failed to get rustc version: {}", e))?;

                let rustc_version = String::from_utf8_lossy(&rustc_output.stdout);

                let info = RustToolchainInfo {
                    cargo_version: cargo_version.stdout.trim().to_string(),
                    rustc_version: rustc_version.trim().to_string(),
                    project_root: self.project_root.to_string_lossy().to_string(),
                    has_cargo_toml: self.project_root.join("Cargo.toml").exists(),
                };

                Ok(serde_json::to_string_pretty(&info)?)
            }

            _ => Err(anyhow!("Unknown Rust compiler tool: {}", tool_name))
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        vec![
            "cargo_build".to_string(),
            "cargo_test".to_string(),
            "cargo_check".to_string(),
            "cargo_clippy".to_string(),
            "cargo_fmt".to_string(),
            "cargo_clean".to_string(),
            "cargo_doc".to_string(),
            "get_cargo_info".to_string(),
        ]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        let description = match tool_name {
            "cargo_build" => "Build the Rust project with Cargo",
            "cargo_test" => "Run tests for the Rust project",
            "cargo_check" => "Check the Rust project for errors without building",
            "cargo_clippy" => "Run Clippy linter on the Rust project",
            "cargo_fmt" => "Format the Rust code using rustfmt",
            "cargo_clean" => "Clean build artifacts",
            "cargo_doc" => "Generate documentation for the project",
            "get_cargo_info" => "Get information about the Rust toolchain and project",
            _ => return None,
        };

        Some(description.to_string())
    }

    fn validate_tool_request(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        // Check if tool is available
        if !self.get_available_tools().contains(&tool_name.to_string()) {
            return Err(anyhow!("Tool '{}' is not available", tool_name));
        }

        // Validate package parameter if present
        if let Some(package_value) = parameters.get("package") {
            if !package_value.is_string() {
                return Err(anyhow!("Package parameter must be a string"));
            }
        }

        // Validate test_name parameter if present
        if let Some(test_name_value) = parameters.get("test_name") {
            if !test_name_value.is_string() {
                return Err(anyhow!("Test name parameter must be a string"));
            }
        }

        // Validate boolean parameters
        for param in &["release", "lib_only", "fix", "check", "open"] {
            if let Some(value) = parameters.get(*param) {
                if !value.is_boolean() {
                    return Err(anyhow!("Parameter '{}' must be a boolean", param));
                }
            }
        }

        Ok(())
    }
}

/// Result of a Cargo command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CargoResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub success: bool,
    pub subcommand: String,
}

/// Information about the Rust toolchain and project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustToolchainInfo {
    pub cargo_version: String,
    pub rustc_version: String,
    pub project_root: String,
    pub has_cargo_toml: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::fs;

    async fn create_test_cargo_project(dir: &std::path::Path) -> Result<()> {
        // Create Cargo.toml
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
        fs::write(dir.join("Cargo.toml"), cargo_toml).await?;

        // Create src directory and main.rs
        fs::create_dir(dir.join("src")).await?;
        let main_rs = r#"
fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
"#;
        fs::write(dir.join("src").join("main.rs"), main_rs).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_cargo_check() {
        let temp_dir = tempdir().unwrap();
        create_test_cargo_project(temp_dir.path()).await.unwrap();

        let executor = RustCompilerExecutor::with_defaults(temp_dir.path().to_path_buf());
        
        let result = executor.execute_tool("cargo_check", &HashMap::new()).await.unwrap();
        
        let cargo_result: CargoResult = serde_json::from_str(&result).unwrap();
        assert_eq!(cargo_result.subcommand, "check");
        // Note: The actual success depends on whether cargo is available in the test environment
    }

    #[tokio::test]
    async fn test_get_cargo_info() {
        let temp_dir = tempdir().unwrap();
        create_test_cargo_project(temp_dir.path()).await.unwrap();

        let executor = RustCompilerExecutor::with_defaults(temp_dir.path().to_path_buf());
        
        let result = executor.execute_tool("get_cargo_info", &HashMap::new()).await.unwrap();
        
        let info: RustToolchainInfo = serde_json::from_str(&result).unwrap();
        assert!(info.has_cargo_toml);
        assert_eq!(info.project_root, temp_dir.path().to_string_lossy());
    }

    #[tokio::test]
    async fn test_validate_cargo_project() {
        let temp_dir = tempdir().unwrap();
        
        // Test without Cargo.toml
        let executor = RustCompilerExecutor::with_defaults(temp_dir.path().to_path_buf());
        assert!(executor.validate_cargo_project().await.is_err());

        // Test with Cargo.toml
        create_test_cargo_project(temp_dir.path()).await.unwrap();
        assert!(executor.validate_cargo_project().await.is_ok());
    }

    #[tokio::test]
    async fn test_cargo_build_with_parameters() {
        let temp_dir = tempdir().unwrap();
        create_test_cargo_project(temp_dir.path()).await.unwrap();

        let executor = RustCompilerExecutor::with_defaults(temp_dir.path().to_path_buf());
        
        let mut params = HashMap::new();
        params.insert("release".to_string(), serde_json::Value::Bool(true));
        
        let result = executor.execute_tool("cargo_build", &params).await.unwrap();
        
        let cargo_result: CargoResult = serde_json::from_str(&result).unwrap();
        assert_eq!(cargo_result.subcommand, "build");
    }

    #[test]
    fn test_parameter_validation() {
        let temp_dir = tempdir().unwrap();
        let executor = RustCompilerExecutor::with_defaults(temp_dir.path().to_path_buf());
        
        // Valid parameters
        let mut valid_params = HashMap::new();
        valid_params.insert("release".to_string(), serde_json::Value::Bool(true));
        valid_params.insert("package".to_string(), serde_json::Value::String("test".to_string()));
        
        assert!(executor.validate_tool_request("cargo_build", &valid_params).is_ok());
        
        // Invalid parameter type
        let mut invalid_params = HashMap::new();
        invalid_params.insert("release".to_string(), serde_json::Value::String("true".to_string()));
        
        assert!(executor.validate_tool_request("cargo_build", &invalid_params).is_err());
    }
}
