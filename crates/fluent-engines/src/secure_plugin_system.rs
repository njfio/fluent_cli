use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use fluent_core::config::EngineConfig;
use fluent_core::traits::Engine;
use fluent_core::types::{ExtractedContent, Request, Response, UpsertRequest, UpsertResponse};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{Mutex, RwLock};

/// Secure plugin system using WebAssembly for sandboxing
///
/// This system provides:
/// - Memory isolation through WASM
/// - Capability-based security
/// - Resource limits and quotas
/// - Cryptographic signature verification
/// - Comprehensive audit logging
/// - Permission-based access control

/// Plugin metadata and manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub engine_type: String,
    pub capabilities: Vec<PluginCapability>,
    pub permissions: PluginPermissions,
    pub signature: Option<String>,
    pub checksum: String,
    pub created_at: String,
    pub expires_at: Option<String>,
}

/// Plugin capabilities that can be requested
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginCapability {
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
    EnvironmentAccess,
    ConfigurationAccess,
    CacheAccess,
    LoggingAccess,
    MetricsAccess,
}

/// Plugin permissions and resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub max_memory_mb: u64,
    pub max_execution_time_ms: u64,
    pub max_network_requests: u32,
    pub allowed_hosts: Vec<String>,
    pub allowed_file_paths: Vec<String>,
    pub max_file_size_mb: u64,
    pub rate_limit_requests_per_minute: u32,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            max_memory_mb: 64,
            max_execution_time_ms: 30000,
            max_network_requests: 100,
            allowed_hosts: vec![],
            allowed_file_paths: vec![],
            max_file_size_mb: 10,
            rate_limit_requests_per_minute: 60,
        }
    }
}

/// Plugin execution context with resource tracking
#[derive(Debug)]
pub struct PluginContext {
    pub plugin_id: String,
    pub permissions: PluginPermissions,
    pub start_time: SystemTime,
    pub memory_used: Arc<Mutex<u64>>,
    pub network_requests_made: Arc<Mutex<u32>>,
    pub files_accessed: Arc<Mutex<Vec<String>>>,
    pub audit_log: Arc<Mutex<Vec<AuditLogEntry>>>,
}

/// Audit log entry for plugin actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: String,
    pub plugin_id: String,
    pub action: String,
    pub resource: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

/// Plugin runtime for executing WASM plugins
pub struct PluginRuntime {
    plugins: Arc<RwLock<HashMap<String, LoadedPlugin>>>,
    plugin_dir: PathBuf,
    signature_verifier: Arc<dyn SignatureVerifier>,
    audit_logger: Arc<dyn AuditLogger>,
}

/// Loaded plugin with WASM instance and metadata
#[allow(dead_code)]
struct LoadedPlugin {
    manifest: PluginManifest,
    wasm_bytes: Vec<u8>,
    context: PluginContext,
    last_used: SystemTime,
    use_count: u64,
}

/// Trait for verifying plugin signatures
#[async_trait]
pub trait SignatureVerifier: Send + Sync {
    async fn verify_signature(&self, plugin_bytes: &[u8], signature: &str) -> Result<bool>;
    async fn get_trusted_keys(&self) -> Result<Vec<String>>;
}

/// Trait for audit logging
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_entry(&self, entry: AuditLogEntry) -> Result<()>;
    async fn get_logs(&self, plugin_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>>;
}

/// Default signature verifier (placeholder for production implementation)
pub struct DefaultSignatureVerifier;

#[async_trait]
impl SignatureVerifier for DefaultSignatureVerifier {
    async fn verify_signature(&self, plugin_bytes: &[u8], signature: &str) -> Result<bool> {
        // Parse the signature from base64
        let signature_bytes = Base64.decode(signature)
            .map_err(|e| anyhow!("Invalid signature format: {}", e))?;

        if signature_bytes.len() != 64 {
            return Err(anyhow!("Invalid signature length: expected 64 bytes, got {}", signature_bytes.len()));
        }

        let signature_array: [u8; 64] = signature_bytes.try_into()
            .map_err(|_| anyhow!("Failed to convert signature to array"))?;
        let signature = Signature::from_bytes(&signature_array);

        // Get trusted public keys
        let trusted_keys = self.get_trusted_keys().await?;

        // Try to verify against each trusted key
        for key_str in trusted_keys {
            if let Ok(key_bytes) = Base64.decode(&key_str) {
                if key_bytes.len() == 32 {
                    if let Ok(public_key) = VerifyingKey::from_bytes(&key_bytes.try_into().unwrap()) {
                        if public_key.verify(plugin_bytes, &signature).is_ok() {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // No valid signature found
        Ok(false)
    }

    async fn get_trusted_keys(&self) -> Result<Vec<String>> {
        // Load trusted public keys from environment or config file
        // For security, we check multiple sources in order of preference

        // 1. Environment variable (for CI/CD and production)
        if let Ok(keys_env) = std::env::var("FLUENT_TRUSTED_KEYS") {
            let keys: Vec<String> = keys_env
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !keys.is_empty() {
                return Ok(keys);
            }
        }

        // 2. Config file (for development and local testing)
        let config_path = std::env::var("HOME")
            .map(|home| PathBuf::from(home).join(".fluent").join("trusted_keys.txt"))
            .unwrap_or_else(|_| PathBuf::from("trusted_keys.txt"));

        if config_path.exists() {
            match tokio::fs::read_to_string(&config_path).await {
                Ok(content) => {
                    let keys: Vec<String> = content
                        .lines()
                        .map(|line| line.trim().to_string())
                        .filter(|line| !line.is_empty() && !line.starts_with('#'))
                        .collect();
                    return Ok(keys);
                }
                Err(e) => {
                    log::warn!("Failed to read trusted keys from {:?}: {}", config_path, e);
                }
            }
        }

        // 3. Default: no trusted keys (secure by default)
        log::warn!("No trusted keys configured. All plugins will be rejected.");
        Ok(vec![])
    }
}

/// Default audit logger
pub struct DefaultAuditLogger {
    log_file: PathBuf,
}

impl DefaultAuditLogger {
    pub fn new(log_file: PathBuf) -> Self {
        Self { log_file }
    }
}

#[async_trait]
impl AuditLogger for DefaultAuditLogger {
    async fn log_entry(&self, entry: AuditLogEntry) -> Result<()> {
        let log_line = serde_json::to_string(&entry)?;
        tokio::fs::write(&self.log_file, format!("{}\n", log_line)).await?;
        Ok(())
    }

    async fn get_logs(&self, plugin_id: &str, limit: usize) -> Result<Vec<AuditLogEntry>> {
        let content = tokio::fs::read_to_string(&self.log_file).await?;
        let mut logs = Vec::new();

        for line in content.lines().rev().take(limit) {
            if let Ok(entry) = serde_json::from_str::<AuditLogEntry>(line) {
                if entry.plugin_id == plugin_id {
                    logs.push(entry);
                }
            }
        }

        logs.reverse();
        Ok(logs)
    }
}

/// Secure plugin engine that wraps WASM plugins
#[allow(dead_code)]
pub struct SecurePluginEngine {
    plugin_id: String,
    runtime: Arc<PluginRuntime>,
    context: Arc<PluginContext>,
}

impl PluginRuntime {
    /// Create a new plugin runtime
    pub fn new(
        plugin_dir: PathBuf,
        signature_verifier: Arc<dyn SignatureVerifier>,
        audit_logger: Arc<dyn AuditLogger>,
    ) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_dir,
            signature_verifier,
            audit_logger,
        }
    }

    /// Load and validate a plugin
    pub async fn load_plugin(&self, plugin_path: &Path) -> Result<String> {
        // Read plugin manifest
        let manifest_path = plugin_path.join("manifest.json");
        let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: PluginManifest = serde_json::from_str(&manifest_content)?;

        // Validate plugin expiration
        if let Some(expires_at) = &manifest.expires_at {
            let expiry = chrono::DateTime::parse_from_rfc3339(expires_at)?;
            if expiry < chrono::Utc::now() {
                return Err(anyhow!("Plugin '{}' has expired", manifest.name));
            }
        }

        // Read WASM binary
        let wasm_path = plugin_path.join("plugin.wasm");
        let wasm_bytes = tokio::fs::read(&wasm_path).await?;

        // Verify checksum
        let actual_checksum = sha2::Sha256::digest(&wasm_bytes);
        let expected_checksum = hex::decode(&manifest.checksum)?;
        if actual_checksum.as_slice() != expected_checksum {
            return Err(anyhow!(
                "Plugin '{}' checksum verification failed",
                manifest.name
            ));
        }

        // Verify signature if present
        if let Some(signature) = &manifest.signature {
            if !self
                .signature_verifier
                .verify_signature(&wasm_bytes, signature)
                .await?
            {
                return Err(anyhow!(
                    "Plugin '{}' signature verification failed",
                    manifest.name
                ));
            }
        } else {
            // Reject unsigned plugins in production
            return Err(anyhow!("Plugin '{}' is not signed", manifest.name));
        }

        // Create plugin context
        let context = PluginContext {
            plugin_id: manifest.name.clone(),
            permissions: manifest.permissions.clone(),
            start_time: SystemTime::now(),
            memory_used: Arc::new(Mutex::new(0)),
            network_requests_made: Arc::new(Mutex::new(0)),
            files_accessed: Arc::new(Mutex::new(Vec::new())),
            audit_log: Arc::new(Mutex::new(Vec::new())),
        };

        // Create loaded plugin
        let loaded_plugin = LoadedPlugin {
            manifest: manifest.clone(),
            wasm_bytes,
            context,
            last_used: SystemTime::now(),
            use_count: 0,
        };

        // Store plugin
        let plugin_id = manifest.name.clone();
        {
            let mut plugins = self.plugins.write().await;
            plugins.insert(plugin_id.clone(), loaded_plugin);
        }

        // Log plugin loading
        let audit_entry = AuditLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            plugin_id: plugin_id.clone(),
            action: "plugin_loaded".to_string(),
            resource: Some(plugin_path.to_string_lossy().to_string()),
            success: true,
            error: None,
        };
        self.audit_logger.log_entry(audit_entry).await?;

        Ok(plugin_id)
    }

    /// Create an engine instance from a loaded plugin
    pub async fn create_engine(
        &self,
        plugin_id: &str,
        config: EngineConfig,
    ) -> Result<Box<dyn Engine>> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        // Check if plugin supports the requested engine type
        if plugin.manifest.engine_type != config.engine {
            return Err(anyhow!(
                "Plugin '{}' does not support engine type '{}'",
                plugin_id,
                config.engine
            ));
        }

        // Create secure plugin engine
        let engine = SecurePluginEngine {
            plugin_id: plugin_id.to_string(),
            runtime: Arc::new(self.clone()),
            context: Arc::new(plugin.context.clone()),
        };

        Ok(Box::new(engine))
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        if plugins.remove(plugin_id).is_some() {
            // Log plugin unloading
            let audit_entry = AuditLogEntry {
                timestamp: chrono::Utc::now().to_rfc3339(),
                plugin_id: plugin_id.to_string(),
                action: "plugin_unloaded".to_string(),
                resource: None,
                success: true,
                error: None,
            };
            self.audit_logger.log_entry(audit_entry).await?;
            Ok(())
        } else {
            Err(anyhow!("Plugin '{}' not found", plugin_id))
        }
    }

    /// List loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.manifest.clone()).collect()
    }

    /// Get plugin statistics
    pub async fn get_plugin_stats(&self, plugin_id: &str) -> Result<PluginStats> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| anyhow!("Plugin '{}' not found", plugin_id))?;

        let memory_used = *plugin.context.memory_used.lock().await;
        let network_requests = *plugin.context.network_requests_made.lock().await;
        let files_accessed = plugin.context.files_accessed.lock().await.len();

        Ok(PluginStats {
            plugin_id: plugin_id.to_string(),
            memory_used_mb: memory_used / 1024 / 1024,
            network_requests_made: network_requests,
            files_accessed_count: files_accessed as u32,
            uptime_seconds: plugin
                .context
                .start_time
                .elapsed()
                .unwrap_or_default()
                .as_secs(),
            use_count: plugin.use_count,
            last_used: plugin.last_used,
        })
    }
}

impl Clone for PluginRuntime {
    fn clone(&self) -> Self {
        Self {
            plugins: Arc::clone(&self.plugins),
            plugin_dir: self.plugin_dir.clone(),
            signature_verifier: Arc::clone(&self.signature_verifier),
            audit_logger: Arc::clone(&self.audit_logger),
        }
    }
}

impl Clone for PluginContext {
    fn clone(&self) -> Self {
        Self {
            plugin_id: self.plugin_id.clone(),
            permissions: self.permissions.clone(),
            start_time: self.start_time,
            memory_used: Arc::clone(&self.memory_used),
            network_requests_made: Arc::clone(&self.network_requests_made),
            files_accessed: Arc::clone(&self.files_accessed),
            audit_log: Arc::clone(&self.audit_log),
        }
    }
}

/// Plugin statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct PluginStats {
    pub plugin_id: String,
    pub memory_used_mb: u64,
    pub network_requests_made: u32,
    pub files_accessed_count: u32,
    pub uptime_seconds: u64,
    pub use_count: u64,
    pub last_used: SystemTime,
}

#[async_trait]
impl Engine for SecurePluginEngine {
    fn execute<'a>(
        &'a self,
        _request: &'a Request,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move {
            // TODO: Execute WASM plugin with request
            // This would involve:
            // 1. Setting up WASM runtime (wasmtime/wasmer)
            // 2. Injecting capabilities based on permissions
            // 3. Monitoring resource usage
            // 4. Enforcing timeouts and limits
            // 5. Logging all actions

            // For now, return a placeholder response
            Err(anyhow!("WASM plugin execution not yet implemented"))
        })
    }

    fn upsert<'a>(
        &'a self,
        _request: &'a UpsertRequest,
    ) -> Box<dyn Future<Output = Result<UpsertResponse>> + Send + 'a> {
        Box::new(async move { Err(anyhow!("Plugin upsert not implemented")) })
    }

    fn get_neo4j_client(&self) -> Option<&Arc<fluent_core::neo4j_client::Neo4jClient>> {
        None
    }

    fn get_session_id(&self) -> Option<String> {
        None
    }

    fn extract_content(&self, _value: &Value) -> Option<ExtractedContent> {
        None
    }

    fn upload_file<'a>(
        &'a self,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<String>> + Send + 'a> {
        Box::new(async move { Err(anyhow!("Plugin file upload not implemented")) })
    }

    fn process_request_with_file<'a>(
        &'a self,
        _request: &'a Request,
        _file_path: &'a Path,
    ) -> Box<dyn Future<Output = Result<Response>> + Send + 'a> {
        Box::new(async move { Err(anyhow!("Plugin file processing not implemented")) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_plugin_runtime_creation() {
        let temp_dir = TempDir::new().unwrap();
        let signature_verifier = Arc::new(DefaultSignatureVerifier);
        let audit_logger = Arc::new(DefaultAuditLogger::new(temp_dir.path().join("audit.log")));

        let runtime = PluginRuntime::new(
            temp_dir.path().to_path_buf(),
            signature_verifier,
            audit_logger,
        );

        let plugins = runtime.list_plugins().await;
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_plugin_permissions_default() {
        let permissions = PluginPermissions::default();
        assert_eq!(permissions.max_memory_mb, 64);
        assert_eq!(permissions.max_execution_time_ms, 30000);
        assert_eq!(permissions.max_network_requests, 100);
    }

    #[test]
    fn test_plugin_capabilities() {
        let capabilities = vec![
            PluginCapability::NetworkAccess,
            PluginCapability::FileSystemRead,
        ];

        assert!(capabilities.contains(&PluginCapability::NetworkAccess));
        assert!(!capabilities.contains(&PluginCapability::FileSystemWrite));
    }
}
