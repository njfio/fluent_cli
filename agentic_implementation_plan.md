# Fluent CLI Agentic Framework Implementation Plan

## Overview

This plan outlines the implementation of real LLM engines, tool executors, and persistent memory for the fluent_cli agentic framework, aligning with existing patterns and architecture.

## 🎯 Phase 1: Real LLM Engine Integration

### 1.1 Align with Existing Configuration Patterns

**Current Pattern Analysis:**
- Uses `CREDENTIAL_` prefix for API keys (e.g., `CREDENTIAL_OPENAI_API_KEY`)
- Engine configs in JSON with connection/parameters structure
- Supports multiple engines: OpenAI, Anthropic, Google Gemini, etc.

**Implementation:**

```rust
// crates/fluent-agent/src/config.rs
use fluent_core::config::{EngineConfig, load_engine_config};
use fluent_engines::create_engine;

pub struct AgentEngineConfig {
    pub reasoning_engine: String,    // "sonnet3.5" 
    pub action_engine: String,       // "gpt-4o"
    pub reflection_engine: String,   // "gemini-flash"
    pub config_path: String,
    pub credentials: HashMap<String, String>,
}

impl AgentEngineConfig {
    pub async fn create_reasoning_engine(&self) -> Result<Box<dyn Engine>> {
        let config = load_engine_config(
            &std::fs::read_to_string(&self.config_path)?,
            &self.reasoning_engine,
            &HashMap::new(),
            &self.credentials,
        )?;
        
        fluent_engines::create_engine(config).await
    }
}
```

## 🔧 Phase 2: Real Tool Integration

### 2.1 File System Tool Executor

```rust
// crates/fluent-agent/src/tools/filesystem.rs
use tokio::fs;

pub struct FileSystemExecutor {
    allowed_paths: Vec<PathBuf>,
    read_only: bool,
}

#[async_trait]
impl ToolExecutor for FileSystemExecutor {
    async fn execute_tool(&self, tool_name: &str, parameters: &HashMap<String, serde_json::Value>) -> Result<String> {
        match tool_name {
            "read_file" => {
                let path = parameters.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing path parameter"))?;
                self.validate_path(path)?;
                let content = fs::read_to_string(path).await?;
                Ok(content)
            }
            "write_file" => {
                if self.read_only { return Err(anyhow!("Write operations not allowed")); }
                let path = parameters.get("path").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing path parameter"))?;
                let content = parameters.get("content").and_then(|v| v.as_str()).ok_or_else(|| anyhow!("Missing content parameter"))?;
                self.validate_path(path)?;
                fs::write(path, content).await?;
                Ok(format!("Successfully wrote {} bytes to {}", content.len(), path))
            }
            _ => Err(anyhow!("Unknown file system tool: {}", tool_name))
        }
    }
}
```

## 💾 Phase 3: Persistent Memory Database

### 3.1 Database Selection: SQLite with sqlx

**Rationale:**
- Async support with sqlx
- Embedded database (no external dependencies)
- ACID compliance for memory consistency
- JSON support for flexible memory storage

```toml
# Add to fluent-agent/Cargo.toml
[dependencies]
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "json", "chrono", "uuid"] }
```

### 3.2 Memory Database Schema

```sql
-- migrations/001_initial.sql
CREATE TABLE IF NOT EXISTS memory_items (
    id TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata JSON,
    importance REAL NOT NULL,
    created_at DATETIME NOT NULL,
    last_accessed DATETIME NOT NULL,
    access_count INTEGER NOT NULL DEFAULT 0,
    tags JSON,
    embedding BLOB
);

CREATE TABLE IF NOT EXISTS episodes (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    context JSON NOT NULL,
    actions_taken JSON NOT NULL,
    outcomes JSON NOT NULL,
    success BOOLEAN NOT NULL,
    lessons_learned JSON NOT NULL,
    occurred_at DATETIME NOT NULL,
    duration_ms INTEGER NOT NULL,
    importance REAL NOT NULL
);

CREATE INDEX idx_memory_type ON memory_items(memory_type);
CREATE INDEX idx_memory_importance ON memory_items(importance);
CREATE INDEX idx_episodes_success ON episodes(success);
```

## 🚀 Phase 4: Integration and CLI Updates

### 4.1 Update CLI Arguments

```rust
// crates/fluent-cli/src/args.rs
#[derive(Parser, Debug)]
pub struct FluentArgs {
    // ... existing args ...
    
    #[arg(long, help = "Enable agentic mode with goal-oriented execution")]
    agentic: bool,
    
    #[arg(long, help = "Goal for the agent to achieve")]
    goal: Option<String>,
    
    #[arg(long, help = "Agent configuration file", default_value = "agent_config.json")]
    agent_config: String,
    
    #[arg(long, help = "Maximum iterations for goal achievement", default_value = "50")]
    max_iterations: u32,
    
    #[arg(long, help = "Enable tool execution (file operations, shell commands)")]
    enable_tools: bool,
}
```

## 📋 Implementation Checklist

### Phase 1: LLM Integration ✅
- [ ] Create AgentEngineConfig struct
- [ ] Integrate with existing fluent-core config system
- [ ] Update LLMReasoningEngine to use real engines
- [ ] Test with OpenAI, Claude, and Gemini

### Phase 2: Tool Integration ✅
- [ ] Implement FileSystemExecutor
- [ ] Implement ShellExecutor  
- [ ] Implement RustCompilerExecutor
- [ ] Add safety validations and sandboxing
- [ ] Create tool registry system

### Phase 3: Persistent Memory ✅
- [ ] Set up SQLite with sqlx
- [ ] Create database migrations
- [ ] Implement SqliteMemoryStore
- [ ] Add memory consolidation logic
- [ ] Test memory persistence across sessions

### Phase 4: CLI Integration ✅
- [ ] Update CLI arguments
- [ ] Add agentic mode to main CLI
- [ ] Create agent configuration templates
- [ ] Add comprehensive error handling
- [ ] Write integration tests

## 🔒 Security Considerations

1. **Tool Sandboxing**: Restrict file operations to allowed directories
2. **Command Validation**: Whitelist allowed shell commands
3. **API Key Security**: Use existing CREDENTIAL_ pattern
4. **Memory Encryption**: Consider encrypting sensitive memory data
5. **Rate Limiting**: Implement LLM API rate limiting

## 📊 Testing Strategy

1. **Unit Tests**: Each tool executor and memory component
2. **Integration Tests**: Full agent execution with real engines
3. **Performance Tests**: Memory database performance
4. **Security Tests**: Tool sandboxing and validation
5. **End-to-End Tests**: Complete goal achievement scenarios

This implementation plan provides a roadmap for transforming the mocked agentic framework into a fully functional system that integrates seamlessly with fluent_cli's existing architecture.
