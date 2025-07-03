# Agent System Guide

## Overview

The Fluent CLI Agent System provides production-ready agentic capabilities with a complete ReAct (Reasoning, Acting, Observing) loop implementation. The system is designed for secure, reliable, and extensible autonomous task execution.

## Architecture

### Core Components

1. **Agent Engine** (`crates/fluent-agent/src/agent.rs`)
   - ReAct loop implementation
   - Goal decomposition and planning
   - Memory management
   - Tool orchestration

2. **Tool System** (`crates/fluent-agent/src/tools/`)
   - File operations with security validation
   - String Replace Editor for surgical file editing
   - Shell command execution with sandboxing
   - Rust compiler integration

3. **Workflow Engine** (`crates/fluent-agent/src/workflow/`)
   - DAG-based execution
   - Step dependencies and error handling
   - Timing and retry logic
   - Parallel execution support

4. **Memory System** (`crates/fluent-agent/src/memory/`)
   - SQLite-based persistent storage
   - Conversation history
   - Goal tracking
   - Performance optimization

## Quick Start

### Basic Agent Usage

```bash
# Interactive agent session
fluent agent --interactive

# Agent with specific goal
fluent agent --agentic --goal "Analyze project structure" --max-iterations 10

# Agent with tools enabled
fluent agent --tools --config agent_config.json
```

### Configuration

Create an `agent_config.json` file:

```json
{
  "max_iterations": 10,
  "enable_tools": true,
  "memory_enabled": true,
  "tool_config": {
    "string_replace_editor": {
      "allowed_paths": ["./src", "./docs"],
      "create_backups": true,
      "case_sensitive": false
    },
    "filesystem": {
      "allowed_paths": ["./"],
      "max_file_size": 10485760
    }
  }
}
```

## Tool System

### String Replace Editor

The String Replace Editor provides surgical file editing capabilities:

#### Features

- **Multiple Occurrence Modes**: First, Last, All, Indexed
- **Line Range Targeting**: Restrict changes to specific line ranges
- **Dry Run Previews**: See changes before applying
- **Automatic Backups**: Timestamped backup creation
- **Security Validation**: Path restrictions and input validation
- **Case Sensitivity Control**: Configurable matching behavior

#### Usage Examples

```bash
# Replace first occurrence
fluent agent --tools --task "Replace 'old_text' with 'new_text' in file.rs"

# Replace in specific line range
fluent agent --tools --task "Replace 'pattern' with 'replacement' in lines 10-20 of file.rs"

# Dry run to preview changes
fluent agent --tools --task "Preview replacing 'old' with 'new' in file.rs"
```

### Available Tools

1. **File Operations**
   - Read files with encoding detection
   - Write files with atomic operations
   - List directories with filtering
   - Create directories with proper permissions

2. **Shell Commands**
   - Execute system commands safely
   - Timeout and resource limits
   - Output capture and streaming
   - Environment variable control

3. **Rust Compiler**
   - Build projects with cargo
   - Run tests and benchmarks
   - Code formatting and linting
   - Dependency management

## Workflow Engine

### Workflow Definition

Define workflows in YAML format:

```yaml
name: "code_analysis_workflow"
description: "Analyze codebase and generate report"

inputs:
  - name: "project_path"
    type: "string"
    required: true

steps:
  - id: "scan_files"
    name: "Scan Project Files"
    tool: "filesystem"
    parameters:
      action: "list_files"
      path: "${inputs.project_path}"
      recursive: true
    
  - id: "analyze_code"
    name: "Analyze Code Quality"
    tool: "rust_compiler"
    parameters:
      action: "check"
      path: "${inputs.project_path}"
    depends_on: ["scan_files"]
    
  - id: "generate_report"
    name: "Generate Analysis Report"
    tool: "string_replace_editor"
    parameters:
      action: "create_file"
      path: "analysis_report.md"
      content: "Analysis results: ${steps.analyze_code.output}"
    depends_on: ["analyze_code"]

outputs:
  - name: "report_path"
    source: "steps.generate_report.output.file_path"
```

### Execution

```bash
# Execute workflow
fluent agent workflow --file analysis_workflow.yaml --input project_path=./src
```

## Security Features

### Input Validation

- All user inputs are validated and sanitized
- Path traversal prevention
- Command injection protection
- File size and type restrictions

### Sandboxing

- Tool execution in isolated environment
- Resource limits (CPU, memory, time)
- Network access control
- File system restrictions

### Audit Trail

- Complete execution logging
- Tool usage tracking
- Error and security event recording
- Performance metrics collection

## Memory System

### Persistent Storage

The agent uses SQLite for persistent memory:

```sql
-- Conversations table
CREATE TABLE conversations (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT
);

-- Messages table
CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id)
);
```

### Memory Operations

```rust
// Store conversation
agent.memory.store_conversation(&conversation).await?;

// Retrieve conversation history
let history = agent.memory.get_conversation_history(&conversation_id).await?;

// Search conversations
let results = agent.memory.search_conversations("query").await?;
```

## Best Practices

### Goal Definition

- Be specific and measurable
- Break down complex goals into sub-goals
- Include success criteria
- Set reasonable iteration limits

### Tool Usage

- Use appropriate tools for each task
- Validate inputs before tool execution
- Handle errors gracefully
- Monitor resource usage

### Security

- Restrict file system access
- Validate all external inputs
- Use sandboxed execution
- Monitor for suspicious activity

## Troubleshooting

### Common Issues

1. **Tool Execution Failures**
   - Check file permissions
   - Verify path restrictions
   - Review security logs

2. **Memory Issues**
   - Check SQLite database integrity
   - Verify disk space
   - Review connection limits

3. **Performance Issues**
   - Monitor iteration counts
   - Check tool execution times
   - Review memory usage

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug fluent agent --interactive
```

## API Reference

See the complete API documentation in the crate documentation:

```bash
cargo doc --open
```
