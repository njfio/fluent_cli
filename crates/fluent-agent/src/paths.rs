use directories::ProjectDirs;
use std::path::PathBuf;

/// Global data directory for Fluent.
///
/// This is OS-specific:
/// - macOS: `~/Library/Application Support/fluent/`
/// - Linux: `~/.local/share/fluent/`
/// - Windows: `%APPDATA%\\fluent\\`
pub fn global_data_dir() -> PathBuf {
    ProjectDirs::from("", "", "fluent")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".fluent"))
}

/// Default global SQLite DB path for agent memory.
pub fn global_agent_memory_db_path() -> PathBuf {
    global_data_dir().join("agent_memory.db")
}
