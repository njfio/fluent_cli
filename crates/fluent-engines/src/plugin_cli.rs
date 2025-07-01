use crate::secure_plugin_system::{
    PluginRuntime, PluginManifest, PluginCapability, PluginPermissions,
    DefaultSignatureVerifier, DefaultAuditLogger
};
use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use sha2::Digest;
use std::path::PathBuf;
use std::sync::Arc;

/// CLI tool for managing Fluent engine plugins
#[derive(Parser)]
#[command(name = "fluent-plugin")]
#[command(about = "A CLI tool for managing Fluent engine plugins")]
pub struct PluginCli {
    /// Plugin directory
    #[arg(short, long, default_value = "./plugins")]
    plugin_dir: PathBuf,

    /// Audit log file
    #[arg(short, long, default_value = "./plugin_audit.log")]
    audit_log: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all loaded plugins
    List,
    /// Load a plugin from directory
    Load {
        /// Plugin directory path
        path: PathBuf,
    },
    /// Unload a plugin
    Unload {
        /// Plugin ID
        plugin_id: String,
    },
    /// Show plugin details
    Show {
        /// Plugin ID
        plugin_id: String,
    },
    /// Show plugin statistics
    Stats {
        /// Plugin ID
        plugin_id: String,
    },
    /// Validate a plugin
    Validate {
        /// Plugin directory path
        path: PathBuf,
    },
    /// Create a new plugin manifest template
    Create {
        /// Plugin name
        name: String,
        /// Engine type
        engine_type: String,
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Show audit logs for a plugin
    Audit {
        /// Plugin ID
        plugin_id: String,
        /// Number of log entries to show
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },
    /// Test plugin security
    SecurityTest {
        /// Plugin directory path
        path: PathBuf,
    },
}

impl PluginCli {
    /// Run the CLI application
    pub async fn run() -> Result<()> {
        let cli = PluginCli::parse();
        
        // Ensure plugin directory exists
        tokio::fs::create_dir_all(&cli.plugin_dir).await?;

        // Create plugin runtime
        let signature_verifier = Arc::new(DefaultSignatureVerifier);
        let audit_logger = Arc::new(DefaultAuditLogger::new(cli.audit_log.clone()));
        let runtime = PluginRuntime::new(cli.plugin_dir.clone(), signature_verifier, audit_logger);

        match cli.command {
            Commands::List => {
                Self::list_plugins(&runtime).await
            }
            Commands::Load { path } => {
                Self::load_plugin(&runtime, &path).await
            }
            Commands::Unload { plugin_id } => {
                Self::unload_plugin(&runtime, &plugin_id).await
            }
            Commands::Show { plugin_id } => {
                Self::show_plugin(&runtime, &plugin_id).await
            }
            Commands::Stats { plugin_id } => {
                Self::show_stats(&runtime, &plugin_id).await
            }
            Commands::Validate { path } => {
                Self::validate_plugin(&path).await
            }
            Commands::Create { name, engine_type, output } => {
                Self::create_plugin_template(&name, &engine_type, &output).await
            }
            Commands::Audit { plugin_id, limit } => {
                Self::show_audit_logs(&runtime, &plugin_id, limit).await
            }
            Commands::SecurityTest { path } => {
                Self::security_test(&path).await
            }
        }
    }

    async fn list_plugins(runtime: &PluginRuntime) -> Result<()> {
        let plugins = runtime.list_plugins().await;
        
        if plugins.is_empty() {
            println!("No plugins loaded.");
            return Ok(());
        }

        println!("🔌 Loaded plugins:");
        for plugin in plugins {
            println!("  • {} v{} ({})", plugin.name, plugin.version, plugin.engine_type);
            println!("    Author: {}", plugin.author);
            println!("    Description: {}", plugin.description);
            println!("    Capabilities: {:?}", plugin.capabilities);
            println!();
        }
        
        Ok(())
    }

    async fn load_plugin(runtime: &PluginRuntime, path: &PathBuf) -> Result<()> {
        println!("🔄 Loading plugin from {}...", path.display());
        
        match runtime.load_plugin(path).await {
            Ok(plugin_id) => {
                println!("✅ Successfully loaded plugin: {}", plugin_id);
            }
            Err(e) => {
                println!("❌ Failed to load plugin: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }

    async fn unload_plugin(runtime: &PluginRuntime, plugin_id: &str) -> Result<()> {
        println!("🔄 Unloading plugin: {}...", plugin_id);
        
        match runtime.unload_plugin(plugin_id).await {
            Ok(()) => {
                println!("✅ Successfully unloaded plugin: {}", plugin_id);
            }
            Err(e) => {
                println!("❌ Failed to unload plugin: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }

    async fn show_plugin(runtime: &PluginRuntime, plugin_id: &str) -> Result<()> {
        let plugins = runtime.list_plugins().await;
        let plugin = plugins.iter()
            .find(|p| p.name == plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        println!("🔌 Plugin: {}", plugin.name);
        println!("Version: {}", plugin.version);
        println!("Engine Type: {}", plugin.engine_type);
        println!("Author: {}", plugin.author);
        println!("Description: {}", plugin.description);
        println!("Created: {}", plugin.created_at);
        
        if let Some(expires_at) = &plugin.expires_at {
            println!("Expires: {}", expires_at);
        }

        println!("\n🔐 Capabilities:");
        for capability in &plugin.capabilities {
            println!("  • {:?}", capability);
        }

        println!("\n⚙️  Permissions:");
        println!("  Max Memory: {} MB", plugin.permissions.max_memory_mb);
        println!("  Max Execution Time: {} ms", plugin.permissions.max_execution_time_ms);
        println!("  Max Network Requests: {}", plugin.permissions.max_network_requests);
        println!("  Rate Limit: {} req/min", plugin.permissions.rate_limit_requests_per_minute);
        
        if !plugin.permissions.allowed_hosts.is_empty() {
            println!("  Allowed Hosts: {:?}", plugin.permissions.allowed_hosts);
        }
        
        if !plugin.permissions.allowed_file_paths.is_empty() {
            println!("  Allowed File Paths: {:?}", plugin.permissions.allowed_file_paths);
        }

        println!("\n🔒 Security:");
        println!("  Signed: {}", plugin.signature.is_some());
        println!("  Checksum: {}", plugin.checksum);
        
        Ok(())
    }

    async fn show_stats(runtime: &PluginRuntime, plugin_id: &str) -> Result<()> {
        let stats = runtime.get_plugin_stats(plugin_id).await?;

        println!("📊 Plugin Statistics: {}", stats.plugin_id);
        println!("Memory Used: {} MB", stats.memory_used_mb);
        println!("Network Requests: {}", stats.network_requests_made);
        println!("Files Accessed: {}", stats.files_accessed_count);
        println!("Uptime: {} seconds", stats.uptime_seconds);
        println!("Use Count: {}", stats.use_count);
        println!("Last Used: {:?}", stats.last_used);
        
        Ok(())
    }

    async fn validate_plugin(path: &PathBuf) -> Result<()> {
        println!("🔍 Validating plugin at {}...", path.display());
        
        // Check manifest exists
        let manifest_path = path.join("manifest.json");
        if !manifest_path.exists() {
            return Err(anyhow!("manifest.json not found"));
        }

        // Parse manifest
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;
        
        println!("✅ Manifest is valid");

        // Check WASM file exists
        let wasm_path = path.join("plugin.wasm");
        if !wasm_path.exists() {
            return Err(anyhow!("plugin.wasm not found"));
        }

        // Verify checksum
        let wasm_bytes = tokio::fs::read(&wasm_path).await?;
        let actual_checksum = sha2::Sha256::digest(&wasm_bytes);
        let expected_checksum = hex::decode(&manifest.checksum)?;
        
        if actual_checksum.as_slice() != expected_checksum {
            return Err(anyhow!("Checksum verification failed"));
        }
        
        println!("✅ Checksum is valid");

        // Check expiration
        if let Some(expires_at) = &manifest.expires_at {
            let expiry = chrono::DateTime::parse_from_rfc3339(expires_at)?;
            if expiry < chrono::Utc::now() {
                return Err(anyhow!("Plugin has expired"));
            }
            println!("✅ Plugin has not expired");
        }

        println!("✅ Plugin validation successful");
        
        Ok(())
    }

    async fn create_plugin_template(name: &str, engine_type: &str, output: &PathBuf) -> Result<()> {
        let plugin_dir = output.join(name);
        tokio::fs::create_dir_all(&plugin_dir).await?;

        // Create manifest template
        let manifest = PluginManifest {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: format!("A {} engine plugin", engine_type),
            author: "Plugin Author".to_string(),
            engine_type: engine_type.to_string(),
            capabilities: vec![
                PluginCapability::NetworkAccess,
                PluginCapability::LoggingAccess,
            ],
            permissions: PluginPermissions::default(),
            signature: None,
            checksum: "placeholder_checksum".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            expires_at: None,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        tokio::fs::write(plugin_dir.join("manifest.json"), manifest_json).await?;

        // Create README template
        let readme = format!(r#"# {} Plugin

## Description
{}

## Engine Type
{}

## Capabilities
- Network Access
- Logging Access

## Building
```bash
# Build the WASM plugin
cargo build --target wasm32-wasi --release

# Copy the WASM file
cp target/wasm32-wasi/release/{}.wasm plugin.wasm

# Update checksum in manifest.json
sha256sum plugin.wasm
```

## Installation
```bash
fluent-plugin load .
```
"#, name, manifest.description, engine_type, name);

        tokio::fs::write(plugin_dir.join("README.md"), readme).await?;

        println!("✅ Created plugin template at {}", plugin_dir.display());
        println!("\n📝 Next steps:");
        println!("  1. Edit manifest.json with your plugin details");
        println!("  2. Implement your plugin in Rust targeting wasm32-wasi");
        println!("  3. Build the WASM binary");
        println!("  4. Update the checksum in manifest.json");
        println!("  5. Sign the plugin (optional but recommended)");
        println!("  6. Load the plugin: fluent-plugin load {}", plugin_dir.display());
        
        Ok(())
    }

    async fn show_audit_logs(runtime: &PluginRuntime, plugin_id: &str, limit: usize) -> Result<()> {
        // This would require access to the audit logger from the runtime
        // For now, just show a placeholder
        println!("📋 Audit logs for plugin '{}' (last {} entries):", plugin_id, limit);
        println!("  [Audit log display not yet implemented]");
        
        Ok(())
    }

    async fn security_test(path: &PathBuf) -> Result<()> {
        println!("🔒 Running security tests for plugin at {}...", path.display());
        
        // Validate plugin first
        Self::validate_plugin(path).await?;
        
        // Additional security checks
        println!("🔍 Checking for security vulnerabilities...");
        
        // Check manifest for suspicious permissions
        let manifest_path = path.join("manifest.json");
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;
        
        // Warn about dangerous capabilities
        let dangerous_capabilities = [
            PluginCapability::FileSystemWrite,
            PluginCapability::EnvironmentAccess,
        ];
        
        for capability in &manifest.capabilities {
            if dangerous_capabilities.contains(capability) {
                println!("⚠️  Warning: Plugin requests dangerous capability: {:?}", capability);
            }
        }
        
        // Check for excessive permissions
        if manifest.permissions.max_memory_mb > 512 {
            println!("⚠️  Warning: Plugin requests high memory limit: {} MB", manifest.permissions.max_memory_mb);
        }
        
        if manifest.permissions.max_execution_time_ms > 60000 {
            println!("⚠️  Warning: Plugin requests long execution time: {} ms", manifest.permissions.max_execution_time_ms);
        }
        
        // Check if plugin is signed
        if manifest.signature.is_none() {
            println!("⚠️  Warning: Plugin is not signed");
        }
        
        println!("✅ Security test completed");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_cli_creation() {
        // Test that the CLI can be created
        let cli = PluginCli::parse_from(&["fluent-plugin", "list"]);
        assert!(matches!(cli.command, Commands::List));
    }
}
