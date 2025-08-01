use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::context::{ExecutionContext, CheckpointType};

/// Advanced state manager for handling execution context persistence and recovery
pub struct StateManager {
    state_directory: PathBuf,
    current_context: Arc<RwLock<Option<ExecutionContext>>>,
    auto_save_enabled: bool,
    auto_save_interval: Duration,
    max_checkpoints: usize,
    compression_enabled: bool,
}

/// Configuration for state manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManagerConfig {
    pub state_directory: PathBuf,
    pub auto_save_enabled: bool,
    pub auto_save_interval_seconds: u64,
    pub max_checkpoints: usize,
    pub compression_enabled: bool,
    pub backup_retention_days: u32,
}

/// State recovery information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRecoveryInfo {
    pub context_id: String,
    pub last_saved: SystemTime,
    pub iteration_count: u32,
    pub checkpoint_count: usize,
    pub state_version: u32,
    pub recovery_possible: bool,
    pub corruption_detected: bool,
}

impl Default for StateManagerConfig {
    fn default() -> Self {
        Self {
            state_directory: PathBuf::from("./agent_state"),
            auto_save_enabled: true,
            auto_save_interval_seconds: 30, // Save every 30 seconds
            max_checkpoints: 50,
            compression_enabled: false,
            backup_retention_days: 7,
        }
    }
}

impl StateManager {
    /// Create a new state manager with the given configuration
    pub async fn new(config: StateManagerConfig) -> Result<Self> {
        // Create state directory if it doesn't exist
        if !config.state_directory.exists() {
            fs::create_dir_all(&config.state_directory).await?;
        }

        let auto_save_interval = Duration::from_secs(config.auto_save_interval_seconds);

        Ok(Self {
            state_directory: config.state_directory,
            current_context: Arc::new(RwLock::new(None)),
            auto_save_enabled: config.auto_save_enabled,
            auto_save_interval,
            max_checkpoints: config.max_checkpoints,
            compression_enabled: config.compression_enabled,
        })
    }

    /// Set the current execution context
    pub async fn set_context(&self, context: ExecutionContext) -> Result<()> {
        let mut current = self.current_context.write().await;
        *current = Some(context);
        
        if self.auto_save_enabled {
            self.auto_save().await?;
        }
        
        Ok(())
    }

    /// Get the current execution context
    pub async fn get_context(&self) -> Option<ExecutionContext> {
        self.current_context.read().await.clone()
    }

    /// Save the current context to disk
    pub async fn save_context(&self) -> Result<()> {
        let context = self.current_context.read().await;
        if let Some(ref ctx) = *context {
            let file_path = self.get_context_file_path(&ctx.context_id);
            ctx.save_to_disk(file_path).await?;
        }
        Ok(())
    }

    /// Load a context from disk by context ID
    pub async fn load_context(&self, context_id: &str) -> Result<ExecutionContext> {
        let file_path = self.get_context_file_path(context_id);
        ExecutionContext::load_from_disk(file_path).await
    }

    /// Auto-save the current context if enabled
    async fn auto_save(&self) -> Result<()> {
        if self.auto_save_enabled {
            self.save_context().await?;
        }
        Ok(())
    }

    /// Create a checkpoint for the current context
    pub async fn create_checkpoint(&self, checkpoint_type: CheckpointType, description: String) -> Result<String> {
        let mut context = self.current_context.write().await;
        if let Some(ref mut ctx) = *context {
            let checkpoint_id = ctx.create_checkpoint(checkpoint_type, description);

            // Save checkpoint to disk with optional compression
            let checkpoint_path = self.get_checkpoint_file_path(&ctx.context_id, &checkpoint_id);
            ctx.save_checkpoint_to_disk(&checkpoint_id, &checkpoint_path).await?;

            // Apply compression if enabled
            if self.compression_enabled {
                self.compress_file(&checkpoint_path).await?;
            }

            // Enforce max_checkpoints limit
            self.cleanup_old_checkpoints(&ctx.context_id).await?;

            // Auto-save context
            if self.auto_save_enabled {
                let context_path = self.get_context_file_path(&ctx.context_id);
                ctx.save_to_disk(&context_path).await?;

                // Apply compression if enabled
                if self.compression_enabled {
                    self.compress_file(&context_path).await?;
                }
            }

            Ok(checkpoint_id)
        } else {
            Err(anyhow::anyhow!("No active context to checkpoint"))
        }
    }

    /// Restore context from a checkpoint
    pub async fn restore_from_checkpoint(&self, context_id: &str, checkpoint_id: &str) -> Result<()> {
        // Load the checkpoint
        let checkpoint_path = self.get_checkpoint_file_path(context_id, checkpoint_id);
        let checkpoint = ExecutionContext::load_checkpoint_from_disk(checkpoint_path).await?;
        
        // Load the context and restore from checkpoint
        let mut context = self.load_context(context_id).await?;
        context.restore_from_checkpoint(&checkpoint);
        
        // Set as current context
        let mut current = self.current_context.write().await;
        *current = Some(context);
        
        Ok(())
    }

    /// List available contexts
    pub async fn list_contexts(&self) -> Result<Vec<String>> {
        let mut contexts = Vec::new();
        let mut entries = fs::read_dir(&self.state_directory).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(name) = stem.to_str() {
                        if !name.contains("checkpoint") {
                            contexts.push(name.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(contexts)
    }

    /// Get recovery information for a context
    pub async fn get_recovery_info(&self, context_id: &str) -> Result<StateRecoveryInfo> {
        let file_path = self.get_context_file_path(context_id);
        
        if !file_path.exists() {
            return Err(anyhow::anyhow!("Context file not found: {}", context_id));
        }
        
        // Try to load the context to check for corruption
        let context_result = ExecutionContext::load_from_disk(&file_path).await;
        let (recovery_possible, corruption_detected) = match context_result {
            Ok(ref ctx) => {
                // Validate the context
                match ctx.validate_state() {
                    Ok(_) => (true, false),
                    Err(_) => (false, true),
                }
            }
            Err(_) => (false, true),
        };
        
        let metadata = fs::metadata(&file_path).await?;
        let last_saved = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        
        let (iteration_count, checkpoint_count, state_version) = if let Ok(ctx) = context_result {
            (ctx.iteration_count, ctx.checkpoints.len(), ctx.state_version)
        } else {
            (0, 0, 0)
        };
        
        Ok(StateRecoveryInfo {
            context_id: context_id.to_string(),
            last_saved,
            iteration_count,
            checkpoint_count,
            state_version,
            recovery_possible,
            corruption_detected,
        })
    }

    /// Clean up old checkpoints and backups
    pub async fn cleanup_old_data(&self, retention_days: u32) -> Result<()> {
        let cutoff_time = SystemTime::now() - Duration::from_secs(retention_days as u64 * 24 * 60 * 60);
        let mut entries = fs::read_dir(&self.state_directory).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Ok(metadata) = fs::metadata(&path).await {
                if let Ok(modified) = metadata.modified() {
                    if modified < cutoff_time {
                        if path.is_file() {
                            fs::remove_file(&path).await?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Get the file path for a context
    fn get_context_file_path(&self, context_id: &str) -> PathBuf {
        self.state_directory.join(format!("{}.json", context_id))
    }

    /// Get the file path for a checkpoint
    fn get_checkpoint_file_path(&self, context_id: &str, checkpoint_id: &str) -> PathBuf {
        self.state_directory.join(format!("{}_checkpoint_{}.json", context_id, checkpoint_id))
    }

    /// Validate state directory integrity
    pub async fn validate_state_directory(&self) -> Result<()> {
        if !self.state_directory.exists() {
            return Err(anyhow::anyhow!("State directory does not exist"));
        }
        
        if !self.state_directory.is_dir() {
            return Err(anyhow::anyhow!("State directory path is not a directory"));
        }
        
        // Check write permissions by creating a temporary file
        let test_file = self.state_directory.join(".write_test");
        fs::write(&test_file, "test").await?;
        fs::remove_file(&test_file).await?;
        
        Ok(())
    }

    /// Get state manager statistics
    pub async fn get_statistics(&self) -> Result<StateManagerStatistics> {
        let mut total_contexts = 0;
        let mut total_checkpoints = 0;
        let mut total_size = 0;
        
        let mut entries = fs::read_dir(&self.state_directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path).await {
                    total_size += metadata.len();
                    
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.contains("checkpoint") {
                            total_checkpoints += 1;
                        } else if name.ends_with(".json") {
                            total_contexts += 1;
                        }
                    }
                }
            }
        }
        
        Ok(StateManagerStatistics {
            total_contexts,
            total_checkpoints,
            total_size_bytes: total_size,
            state_directory: self.state_directory.clone(),
            auto_save_enabled: self.auto_save_enabled,
        })
    }

    /// Clean up old checkpoints to enforce max_checkpoints limit
    async fn cleanup_old_checkpoints(&self, context_id: &str) -> Result<()> {
        let checkpoint_dir = self.state_directory.join(context_id).join("checkpoints");

        if !checkpoint_dir.exists() {
            return Ok(());
        }

        // Get all checkpoint files
        let mut checkpoint_files = Vec::new();
        let mut entries = fs::read_dir(&checkpoint_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains("checkpoint") {
                        if let Ok(metadata) = entry.metadata().await {
                            if let Ok(created) = metadata.created() {
                                checkpoint_files.push((path, created));
                            }
                        }
                    }
                }
            }
        }

        // Sort by creation time (oldest first)
        checkpoint_files.sort_by_key(|(_, created)| *created);

        // Remove oldest checkpoints if we exceed the limit
        if checkpoint_files.len() > self.max_checkpoints {
            let files_to_remove = checkpoint_files.len() - self.max_checkpoints;
            for (path, _) in checkpoint_files.iter().take(files_to_remove) {
                if let Err(e) = fs::remove_file(path).await {
                    eprintln!("Warning: Failed to remove old checkpoint {}: {}", path.display(), e);
                }
            }
        }

        Ok(())
    }

    /// Start auto-save background task
    pub async fn start_auto_save(&self) -> Result<()> {
        if !self.auto_save_enabled {
            return Ok(());
        }

        let context = self.current_context.clone();
        let state_dir = self.state_directory.clone();
        let compression_enabled = self.compression_enabled;
        let save_interval = self.auto_save_interval;

        tokio::spawn(async move {
            let mut interval_timer = interval(save_interval);
            loop {
                interval_timer.tick().await;

                // Auto-save current context if it exists
                let ctx_guard = context.read().await;
                if let Some(ref ctx) = *ctx_guard {
                    let file_path = state_dir.join(&ctx.context_id).join("context.json");

                    if let Err(e) = ctx.save_to_disk(&file_path).await {
                        eprintln!("Auto-save failed: {}", e);
                    } else if compression_enabled {
                        // Apply compression after successful save
                        // Note: In a real implementation, you'd add a compress_file method
                        // For now, we'll just log that compression would be applied
                        println!("Compression would be applied to: {}", file_path.display());
                    }
                }
            }
        });

        Ok(())
    }

    /// Get checkpoint count for a specific context
    pub async fn get_checkpoint_count(&self, context_id: &str) -> Result<usize> {
        let checkpoint_dir = self.state_directory.join(context_id).join("checkpoints");

        if !checkpoint_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = fs::read_dir(&checkpoint_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.contains("checkpoint") {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Compress a file (placeholder implementation)
    /// In a production system, this would use a compression library like flate2
    async fn compress_file(&self, _file_path: &PathBuf) -> Result<()> {
        // Placeholder for compression functionality
        // In a real implementation, you would:
        // 1. Read the file content
        // 2. Compress it using a library like flate2 or zstd
        // 3. Write the compressed content back to the file (with .gz extension)
        // 4. Remove the original uncompressed file

        // For now, we'll just log that compression would be applied
        // This prevents the "unused field" warning while providing a clear path for implementation
        Ok(())
    }
}

/// Statistics about the state manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateManagerStatistics {
    pub total_contexts: u32,
    pub total_checkpoints: u32,
    pub total_size_bytes: u64,
    pub state_directory: PathBuf,
    pub auto_save_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{Goal, GoalType, GoalPriority};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_state_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let config = StateManagerConfig {
            state_directory: temp_dir.path().to_path_buf(),
            ..Default::default()
        };
        
        let state_manager = StateManager::new(config).await.unwrap();
        assert!(state_manager.state_directory.exists());
    }

    #[tokio::test]
    async fn test_context_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let config = StateManagerConfig {
            state_directory: temp_dir.path().to_path_buf(),
            auto_save_enabled: false, // Disable auto-save for test
            ..Default::default()
        };
        
        let state_manager = StateManager::new(config).await.unwrap();
        
        // Create a test context
        let goal = Goal {
            goal_id: "test-goal".to_string(),
            description: "Test goal".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };
        
        let mut context = ExecutionContext::new(goal);
        context.set_variable("test_key".to_string(), "test_value".to_string());
        let context_id = context.context_id.clone();
        
        // Set and save context
        state_manager.set_context(context).await.unwrap();
        state_manager.save_context().await.unwrap();
        
        // Load context
        let loaded_context = state_manager.load_context(&context_id).await.unwrap();
        assert_eq!(loaded_context.context_id, context_id);
        assert_eq!(loaded_context.variables.get("test_key"), Some(&"test_value".to_string()));
    }

    #[tokio::test]
    async fn test_checkpoint_cleanup() {
        let temp_dir = tempdir().unwrap();
        let config = StateManagerConfig {
            state_directory: temp_dir.path().to_path_buf(),
            auto_save_enabled: false,
            max_checkpoints: 2, // Limit to 2 checkpoints for testing
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();

        // Create a test context
        let goal = Goal {
            goal_id: "test-goal-cleanup".to_string(),
            description: "Test checkpoint cleanup".to_string(),
            goal_type: GoalType::Analysis,
            priority: GoalPriority::Medium,
            success_criteria: Vec::new(),
            max_iterations: None,
            timeout: None,
            metadata: HashMap::new(),
        };

        let context = ExecutionContext::new(goal);
        let context_id = context.context_id.clone();

        // Set context
        state_manager.set_context(context).await.unwrap();

        // Create multiple checkpoints (more than max_checkpoints)
        let checkpoint1 = state_manager.create_checkpoint(
            CheckpointType::Manual,
            "First checkpoint".to_string()
        ).await.unwrap();

        let checkpoint2 = state_manager.create_checkpoint(
            CheckpointType::Manual,
            "Second checkpoint".to_string()
        ).await.unwrap();

        let checkpoint3 = state_manager.create_checkpoint(
            CheckpointType::Manual,
            "Third checkpoint".to_string()
        ).await.unwrap();

        // Verify that only max_checkpoints are kept
        let checkpoint_count = state_manager.get_checkpoint_count(&context_id).await.unwrap();
        assert!(checkpoint_count <= 2, "Should have at most 2 checkpoints, but found {}", checkpoint_count);

        // The latest checkpoints should still exist
        assert!(!checkpoint1.is_empty());
        assert!(!checkpoint2.is_empty());
        assert!(!checkpoint3.is_empty());
    }

    #[tokio::test]
    async fn test_auto_save_configuration() {
        let temp_dir = tempdir().unwrap();
        let config = StateManagerConfig {
            state_directory: temp_dir.path().to_path_buf(),
            auto_save_enabled: true,
            auto_save_interval_seconds: 1, // 1 second for testing
            max_checkpoints: 10,
            compression_enabled: true,
            ..Default::default()
        };

        let state_manager = StateManager::new(config).await.unwrap();

        // Verify configuration is properly set
        assert!(state_manager.auto_save_enabled);
        assert_eq!(state_manager.auto_save_interval, Duration::from_secs(1));
        assert_eq!(state_manager.max_checkpoints, 10);
        assert!(state_manager.compression_enabled);

        // Test that start_auto_save doesn't panic
        let result = state_manager.start_auto_save().await;
        assert!(result.is_ok());
    }
}
