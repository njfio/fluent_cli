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

5. **State Management** (`crates/fluent-agent/src/state_manager.rs`)
   - Execution context persistence
   - Checkpoint creation and restoration
   - State validation and recovery
   - Auto-save capabilities

6. **Self-Reflection Engine** (`crates/fluent-agent/src/reflection.rs`)
   - Advanced self-reflection and learning
   - Strategy adjustment mechanisms
   - Performance pattern analysis
   - Learning experience retention

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

## State Management System

### Execution Context and Persistence

The state management system provides comprehensive execution context tracking with persistence capabilities:

#### Key Features

- **Execution Context**: Maintains complete state throughout agent execution
- **Checkpoints**: Create snapshots at critical execution points
- **State Persistence**: Save and restore execution state to/from disk
- **Recovery**: Handle failures and resume from checkpoints
- **Validation**: Ensure state consistency and integrity

#### Configuration

```rust
use fluent_agent::{StateManager, StateManagerConfig};

let config = StateManagerConfig {
    state_directory: PathBuf::from("./agent_state"),
    auto_save_enabled: true,
    auto_save_interval_seconds: 30,
    max_checkpoints: 50,
    compression_enabled: false,
    backup_retention_days: 7,
};

let state_manager = StateManager::new(config).await?;
```

#### Checkpoint Management

```rust
// Create manual checkpoint
let checkpoint_id = context.create_checkpoint(
    CheckpointType::Manual,
    "Important milestone reached".to_string()
);

// Automatic checkpoints are created every N iterations
context.set_auto_checkpoint_interval(Some(5));

// Restore from checkpoint
context.restore_from_checkpoint(&checkpoint);
```

#### State Persistence

```rust
// Save execution context to disk
context.save_to_disk("./context.json").await?;

// Load execution context from disk
let context = ExecutionContext::load_from_disk("./context.json").await?;

// State manager operations
state_manager.set_context(context).await?;
state_manager.save_context().await?;
let loaded_context = state_manager.load_context("context_id").await?;
```

#### State Validation

```rust
// Validate state consistency
match context.validate_state() {
    Ok(_) => println!("State is valid"),
    Err(e) => println!("State validation failed: {}", e),
}

// Get recovery information
let recovery_info = state_manager.get_recovery_info("context_id").await?;
println!("Recovery possible: {}", recovery_info.recovery_possible);
```

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

## Self-Reflection & Strategy Adjustment System

### Advanced Learning and Adaptation

The self-reflection system provides sophisticated learning and adaptation capabilities that enable the agent to continuously improve its performance through experience.

#### Key Features

- **Multi-Type Reflection**: Routine, triggered, deep, meta, and crisis reflection modes
- **Strategy Adjustment**: Automatic strategy optimization based on performance analysis
- **Learning Retention**: Experience-based learning with configurable retention periods
- **Pattern Recognition**: Success and failure pattern identification
- **Performance Metrics**: Comprehensive performance tracking and analysis

#### Reflection Configuration

```rust
use fluent_agent::{ReflectionEngine, ReflectionConfig};

let config = ReflectionConfig {
    reflection_frequency: 5,           // Reflect every 5 iterations
    deep_reflection_frequency: 20,     // Deep reflection every 20 reflections
    learning_retention_days: 30,       // Keep learning experiences for 30 days
    confidence_threshold: 0.6,         // Trigger reflection if confidence drops below 0.6
    performance_threshold: 0.7,        // Trigger adjustment if performance drops below 0.7
    enable_meta_reflection: true,      // Enable reflection on reflection process
    strategy_adjustment_sensitivity: 0.8, // How readily to adjust strategy
};

let mut reflection_engine = ReflectionEngine::with_config(config);
```

#### Reflection Types

```rust
// Automatic reflection triggers
if let Some(trigger) = reflection_engine.should_reflect(&context) {
    let reflection_result = reflection_engine.reflect(
        &context,
        &reasoning_engine,
        trigger
    ).await?;

    // Apply strategy adjustments
    for adjustment in &reflection_result.strategy_adjustments {
        apply_strategy_adjustment(adjustment).await?;
    }
}

// Manual reflection
let manual_reflection = reflection_engine.reflect(
    &context,
    &reasoning_engine,
    ReflectionTrigger::UserRequest
).await?;
```

#### Strategy Adjustment Types

The system can generate various types of strategy adjustments:

- **Goal Refinement**: Adjust goal parameters and success criteria
- **Task Prioritization**: Reorder tasks based on importance and dependencies
- **Tool Selection**: Choose more effective tools for specific tasks
- **Approach Modification**: Change the overall approach to problem-solving
- **Resource Reallocation**: Redistribute time and computational resources
- **Timeline Adjustment**: Modify deadlines and scheduling
- **Quality Standards**: Adjust quality thresholds and validation criteria
- **Risk Management**: Implement risk mitigation strategies

#### Learning Insights

The reflection system generates actionable learning insights:

```rust
// Access learning insights from reflection
for insight in &reflection_result.learning_insights {
    match insight.insight_type {
        InsightType::SuccessFactors => {
            // Apply successful patterns to future tasks
            apply_success_pattern(&insight).await?;
        }
        InsightType::FailureFactors => {
            // Implement failure prevention measures
            implement_failure_prevention(&insight).await?;
        }
        InsightType::PerformancePattern => {
            // Optimize performance based on patterns
            optimize_performance(&insight).await?;
        }
        _ => {}
    }
}
```

#### Performance Metrics

The system tracks comprehensive performance metrics:

```rust
let stats = reflection_engine.get_reflection_statistics();
println!("Learning Experiences: {}", stats.total_learning_experiences);
println!("Strategy Patterns: {}", stats.total_strategy_patterns);
println!("Average Success Rate: {:.2}", stats.average_success_rate);
println!("Learning Velocity: {:.2}", stats.learning_velocity);
```

#### Integration with Orchestrator

The reflection system is fully integrated with the agent orchestrator:

```rust
// Automatic reflection during execution
let orchestrator = AgentOrchestrator::new(
    reasoning_engine,
    action_planner,
    action_executor,
    observation_processor,
    memory_system,
    persistent_state_manager,
    reflection_engine, // Integrated reflection engine
).await;

// Reflection happens automatically during goal execution
let result = orchestrator.execute_goal(goal).await?;

// Manual reflection trigger
let reflection_result = orchestrator.trigger_reflection(
    &context,
    "User requested analysis".to_string()
).await?;
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
