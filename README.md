# Fluent CLI - Advanced Multi-LLM Command Line Interface

A modern, secure, and modular Rust-based command-line interface for interacting with multiple Large Language Model (LLM) providers. Fluent CLI provides a unified interface for OpenAI, Anthropic, Google Gemini, and other LLM services, with production-ready agentic capabilities, comprehensive security features, and full Model Context Protocol (MCP) integration.

## 🎉 **Recent Major Updates (v0.3.0)**

### 🔒 **Security & Stability Improvements**

- **Enhanced Error Handling**: Significantly reduced `unwrap()` calls with proper error handling (work in progress)
- **Command Injection Protection**: Comprehensive input validation and command sanitization
- **Path Traversal Prevention**: Secure file operations with strict path validation
- **Memory Safety**: Eliminated unsafe operations and improved memory management
- **Credential Security**: Secure memory clearing and proper credential management

### 🏗️ **Architecture & Performance**

- **Modular Codebase**: Ongoing refactoring of large monolithic functions into focused modules
- **Connection Pooling**: HTTP client reuse and connection management
- **Response Caching**: Intelligent caching system with configurable TTL
- **Async Optimization**: Proper async/await patterns throughout the codebase
- **Memory Optimization**: Reduced allocations and improved resource management

### 🤖 **Enhanced Agentic Capabilities**

- **ReAct Agent Loop**: Complete reasoning, acting, observing cycle implementation
- **Advanced Tool System**: Secure file operations, shell commands, and code analysis
- **Workflow Engine**: DAG-based execution with proper timing and retry logic
- **String Replace Editor**: Surgical file editing with comprehensive test coverage
- **MCP Integration**: Full Model Context Protocol client and server support
- **Self-Reflection Engine**: Advanced learning and strategy adjustment capabilities
- **State Management**: Execution context persistence with checkpoint/restore functionality

### 📊 **Quality & Testing**

- **Clean Builds**: Minimal warnings and errors in builds (ongoing improvements)
- **Growing Test Coverage**: Expanding unit and integration test coverage
- **Dependency Management**: Pinned critical dependencies for stability
- **Documentation**: Comprehensive API documentation and usage examples

### ⚠️ **Current Limitations**

- **Work in Progress**: Some features are still under development (marked with TODO)
- **Test Coverage**: Test coverage is expanding but not yet comprehensive
- **Error Handling**: Ongoing migration from `unwrap()` to proper error handling
- **Binary Structure**: Consolidating dual binary structure for consistency

## 🚀 Key Features

### 🌐 **Multi-Provider LLM Support**

- **OpenAI**: GPT models with text and vision capabilities
- **Anthropic**: Claude models for advanced reasoning
- **Google**: Gemini Pro for multimodal interactions
- **Additional Providers**: Cohere, Mistral, Perplexity, Groq, and more
- **Webhook Integration**: Custom API endpoints and local models

### 🔧 **Core Functionality**

- **Direct LLM Queries**: Send text prompts to any supported LLM provider
- **Image Analysis**: Vision capabilities for supported models
- **Configuration Management**: YAML-based configuration for multiple engines
- **Pipeline Execution**: YAML-defined multi-step workflows
- **Caching**: Optional request caching for improved performance

### 🤖 **Production-Ready Agentic Features**

- **Modular Agent Architecture**: Clean separation of reasoning, action, and reflection engines
- **MCP Integration**: Full Model Context Protocol client and server capabilities
- **Advanced Tool System**: Secure file operations, shell commands, and code analysis
- **String Replace Editor**: Surgical file editing with precision targeting and validation
- **Memory System**: SQLite-based persistent memory with performance optimization
- **Security Sandboxing**: Rate limiting, input validation, and secure execution environment

### 🧠 **Self-Reflection & Learning System**

- **Multi-Type Reflection**: Routine, triggered, deep, meta, and crisis reflection modes
- **Strategy Adjustment**: Automatic strategy optimization based on performance analysis
- **Learning Retention**: Experience-based learning with configurable retention periods
- **Pattern Recognition**: Success and failure pattern identification and application
- **Performance Metrics**: Comprehensive performance tracking and confidence assessment
- **State Persistence**: Execution context and learning experience persistence

### 🔒 **Security & Quality Features**

- **Comprehensive Input Validation**: Protection against injection attacks and malicious input
- **Rate Limiting**: Configurable request throttling (30 requests/minute default)
- **Command Sandboxing**: Isolated execution environment with timeouts
- **Security Audit Tools**: Automated security scanning and vulnerability detection
- **Code Quality Assessment**: Automated quality metrics and best practice validation

## 📦 Installation

### From Source

```bash
git clone https://github.com/njfio/fluent_cli.git
cd fluent_cli
cargo build --release
```

## 🚀 Quick Start

### 1. Configure API Keys

```bash
# Set your preferred LLM provider API key
export OPENAI_API_KEY="your-api-key-here"
# or
export ANTHROPIC_API_KEY="your-api-key-here"
```

### 2. Basic Usage

#### Direct LLM Queries

```bash
# Simple query to OpenAI
fluent openai "Explain quantum computing"

# Query with Anthropic
fluent anthropic "Write a Python function to calculate fibonacci"

# Query with image (vision models)
fluent openai "What's in this image?" --upload_image_file image.jpg

# Enable caching for repeated queries
fluent openai "Complex analysis task" --cache
```

### 3. New Modular Command Structure

#### Agent Commands

```bash
# Interactive agent session (requires API keys)
fluent openai agent

# Agentic mode with specific goal (requires API keys)
fluent openai --agentic --goal "Build a simple web server" --max-iterations 10 --enable-tools

# Agent with MCP capabilities (requires API keys)
fluent agent-mcp --engine openai --task "Analyze codebase" --mcp-servers "filesystem:mcp-server-filesystem"

# Note: Set appropriate API keys before running:
# export OPENAI_API_KEY="your-api-key-here"
# export ANTHROPIC_API_KEY="your-api-key-here"
```

#### Pipeline Commands

```bash
# Execute a pipeline
fluent pipeline --file pipeline.yaml --input "process this data"

# Pipeline with JSON output
fluent pipeline --file pipeline.yaml --json-output

# Force fresh execution (ignore cache)
fluent pipeline --file pipeline.yaml --force-fresh
```

#### MCP (Model Context Protocol) Commands

```bash
# Start MCP server
fluent mcp server

# Run agent with MCP integration
fluent mcp agent --engine openai --task "analyze codebase" --servers server1,server2
```

#### Neo4j Integration Commands

```bash
# Query Neo4j with natural language
fluent neo4j query "Find all connected nodes" --engine openai

# Upsert data to Neo4j
fluent neo4j upsert --data "user data" --engine anthropic
```

#### Direct Engine Commands (Legacy Support)

```bash
# Direct engine queries (still supported)
fluent openai "Explain quantum computing"

# Execute a pipeline
fluent openai pipeline -f pipeline.yaml -i "input data"

# Start MCP server
fluent openai mcp

# Agent with MCP capabilities (experimental)
fluent openai agent-mcp -e openai -t "Analyze files" -s "filesystem:mcp-server-filesystem"
```

## 🔧 Configuration

### Engine Configuration

Create a YAML configuration file for your LLM providers:

```yaml
# config.yaml
engines:
  - name: "openai-gpt4"
    engine: "openai"
    model: "gpt-4"
    api_key: "${OPENAI_API_KEY}"
    max_tokens: 4000
    temperature: 0.7

  - name: "claude-3"
    engine: "anthropic"
    model: "claude-3-sonnet-20240229"
    api_key: "${ANTHROPIC_API_KEY}"
    max_tokens: 4000
    temperature: 0.5
```

### Pipeline Configuration

Define multi-step workflows in YAML:

```yaml
# pipeline.yaml
name: "code-analysis"
description: "Analyze code and generate documentation"
steps:
  - name: "read-files"
    type: "file_operation"
    config:
      operation: "read"
      pattern: "src/**/*.rs"

  - name: "analyze"
    type: "llm_query"
    config:
      engine: "openai"
      prompt: "Analyze this code and suggest improvements: {{previous_output}}"
```

### Self-Reflection Configuration

Configure the agent's self-reflection and learning capabilities:

```yaml
# reflection_config.yaml
reflection:
  reflection_frequency: 5              # Reflect every 5 iterations
  deep_reflection_frequency: 20        # Deep reflection every 20 reflections
  learning_retention_days: 30          # Keep learning experiences for 30 days
  confidence_threshold: 0.6            # Trigger reflection if confidence < 0.6
  performance_threshold: 0.7           # Trigger adjustment if performance < 0.7
  enable_meta_reflection: true         # Enable reflection on reflection process
  strategy_adjustment_sensitivity: 0.8 # How readily to adjust strategy (0.0-1.0)

state_management:
  state_directory: "./agent_state"     # Directory for state persistence
  auto_save_enabled: true              # Enable automatic state saving
  auto_save_interval_seconds: 30       # Save state every 30 seconds
  max_checkpoints: 50                  # Maximum checkpoints to retain
  backup_retention_days: 7             # Keep backups for 7 days
```

### Agent Configuration

Complete agent configuration with all capabilities:

```yaml
# agent_config.yaml
agent:
  max_iterations: 20
  enable_tools: true
  memory_enabled: true
  reflection_enabled: true

reasoning:
  engine: "openai"
  model: "gpt-4"
  temperature: 0.7

tools:
  string_replace_editor:
    allowed_paths: ["./src", "./docs", "./examples"]
    create_backups: true
    case_sensitive: false
    max_file_size: 10485760  # 10MB

  filesystem:
    allowed_paths: ["./"]
    max_file_size: 10485760

  shell:
    allowed_commands: ["cargo", "git", "ls", "cat"]
    timeout_seconds: 30
```

## 🤖 Experimental Features

### Agent Mode

Interactive agent sessions with basic memory and tool access:

```bash
# Start an interactive agent session (requires OPENAI_API_KEY)
fluent openai agent

# Agent with specific goal (requires OPENAI_API_KEY)
fluent openai --agentic --goal "Analyze project structure" --enable-tools
```

### MCP Integration

Basic Model Context Protocol support:

```bash
# Start MCP server
fluent openai mcp

# Agent with MCP (experimental)
fluent openai agent-mcp -e openai -t "Read files" -s "filesystem:server"
```

**Note**: Agentic features are experimental and under active development.

## 🔧 Tool System

### String Replace Editor

Advanced file editing capabilities with surgical precision:

```bash
# Replace first occurrence
fluent openai agent --tool string_replace --file "src/main.rs" --old "println!" --new "log::info!" --occurrence "First"

# Replace all occurrences with backup
fluent openai agent --tool string_replace --file "config.toml" --old "debug = false" --new "debug = true" --occurrence "All" --backup

# Line range replacement (lines 10-20 only)
fluent openai agent --tool string_replace --file "lib.rs" --old "i32" --new "u32" --line-range "10,20"

# Dry run preview
fluent openai agent --tool string_replace --file "app.rs" --old "HashMap" --new "BTreeMap" --dry-run
```

**Features:**

- **Multiple occurrence modes**: First, Last, All, Indexed
- **Line range targeting**: Restrict changes to specific line ranges
- **Dry run previews**: See changes before applying
- **Automatic backups**: Timestamped backup creation
- **Security validation**: Path restrictions and input validation
- **Case sensitivity control**: Configurable matching behavior

### Available Tools

- **File Operations**: Read, write, list, create directories
- **String Replace Editor**: Surgical file editing with precision targeting
- **Shell Commands**: Execute system commands safely
- **Rust Compiler**: Build, test, check, clippy, format
- **Git Operations**: Basic version control operations

## 🛠️ Supported Engines

### Available Providers

- **OpenAI**: GPT-3.5, GPT-4, GPT-4 Turbo, GPT-4 Vision
- **Anthropic**: Claude 3 (Haiku, Sonnet, Opus), Claude 2.1
- **Google**: Gemini Pro, Gemini Pro Vision
- **Cohere**: Command, Command Light, Command Nightly
- **Mistral**: Mistral 7B, Mistral 8x7B, Mistral Large
- **Perplexity**: Various models via API
- **Groq**: Fast inference models
- **Custom**: Webhook endpoints for local/custom models

### Configuration

Set API keys as environment variables:

```bash
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"
export GOOGLE_API_KEY="your-key"
# ... etc
```

## 🔧 Development Status

### Current State

- **Core LLM Integration**: ✅ Fully functional
- **Multi-provider Support**: ✅ Working with major providers
- **Basic Pipeline System**: ✅ YAML-based workflows
- **Configuration Management**: ✅ YAML configuration files
- **Caching System**: ✅ Optional request caching

### Production-Ready Features

- **Agent System**: ✅ Complete ReAct loop implementation
- **MCP Integration**: ✅ Full client and server support
- **Advanced Tool System**: ✅ Production-ready file operations and code analysis
- **String Replace Editor**: ✅ Surgical file editing with precision targeting
- **Memory System**: ✅ SQLite-based persistent memory with optimization
- **Self-Reflection Engine**: ✅ Advanced learning and strategy adjustment
- **State Management**: ✅ Execution context persistence with checkpoint/restore

### Planned Features

- Enhanced multi-modal capabilities
- Expanded tool ecosystem
- Advanced workflow orchestration
- Real-time collaboration features
- Plugin system for custom tools

## 🧪 Development

### Building from Source

```bash
git clone https://github.com/njfio/fluent_cli.git
cd fluent_cli
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test --package fluent-agent

# Run integration tests
cargo test --test integration

# Run reflection system tests
cargo test -p fluent-agent reflection
```

### Running Examples

```bash
# Run the self-reflection and strategy adjustment demo
cargo run --example reflection_demo

# Run the state management demo
cargo run --example state_management_demo

# Run the string replace editor demo
cargo run --example string_replace_demo

# Run other available examples
cargo run --example real_agentic_demo
cargo run --example working_agentic_demo
```

### Quality Assurance Tools

#### Security Audit

```bash
# Run comprehensive security audit (15 security checks)
./scripts/security_audit.sh
```

#### Code Quality Assessment

```bash
# Run code quality checks (15 quality metrics)
./scripts/code_quality_check.sh
```

### Project Structure

```text
fluent_cli/
├── crates/
│   ├── fluent-cli/          # Main CLI application with modular commands
│   ├── fluent-core/         # Core utilities and configuration
│   ├── fluent-engines/      # LLM engine implementations
│   ├── fluent-agent/        # Agentic capabilities and tools
│   ├── fluent-storage/      # Storage and persistence layer
│   └── fluent-sdk/          # SDK for external integrations
├── docs/                    # Organized documentation
│   ├── analysis/           # Code review and analysis
│   ├── guides/             # User and development guides
│   ├── implementation/     # Implementation status
│   ├── security/           # Security documentation
│   └── testing/            # Testing documentation
├── scripts/                # Quality assurance scripts
├── tests/                  # Integration tests and test data
└── examples/               # Usage examples and demos
```

## 🤝 Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **GitHub Issues**: [Report bugs or request features](https://github.com/njfio/fluent_cli/issues)
- **Discussions**: [Community discussions](https://github.com/njfio/fluent_cli/discussions)

---

**Fluent CLI: Multi-LLM Command Line Interface** 🚀
