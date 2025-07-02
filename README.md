# Fluent CLI - Advanced Agentic Development Platform

A cutting-edge command-line interface that transforms traditional LLM interactions into a powerful agentic development platform. Fluent CLI combines multiple LLM providers with advanced agent capabilities, persistent memory, tool orchestration, and Model Context Protocol (MCP) integration.

## 🚀 Key Features

### 🤖 **Agentic AI System**
- **Autonomous Agents**: Self-directed AI agents that can plan, execute, and learn from complex tasks
- **Persistent Memory**: SQLite-based long-term memory system for agent learning and adaptation
- **Tool Orchestration**: Dynamic tool discovery, selection, and execution
- **Goal-Oriented Execution**: Agents that work towards specific objectives with multi-step planning

### 🔌 **Model Context Protocol (MCP) Integration**
- **MCP Client**: Connect to and use tools from any MCP-compatible server
- **Multi-Server Support**: Manage multiple MCP server connections simultaneously
- **AI-Powered Tool Selection**: Intelligent reasoning about which tools to use for specific tasks
- **Ecosystem Integration**: Compatible with VS Code, Claude Desktop, and community MCP servers

### 🧠 **Advanced Reasoning Engine**
- **Chain-of-Thought Processing**: Sophisticated reasoning capabilities for complex problem solving
- **Context-Aware Decision Making**: Agents that understand and adapt to their environment
- **Learning and Adaptation**: Continuous improvement through experience and feedback
- **Multi-Modal Processing**: Support for text, vision, and document analysis

### 🛠️ **Comprehensive Tool System**
- **File System Operations**: Read, write, search, and manipulate files and directories
- **Command Execution**: Safe execution of shell commands with result capture
- **Code Analysis**: Advanced code understanding and manipulation capabilities
- **Dynamic Tool Registry**: Extensible architecture for adding new tool categories

### 🌐 **Multi-Provider LLM Support**
- **OpenAI**: GPT-3.5, GPT-4, GPT-4 Turbo, GPT-4 Vision, DALL-E
- **Anthropic**: Claude 3 (Haiku, Sonnet, Opus), Claude 2.1, Claude Instant
- **Google**: Gemini Pro, Gemini Pro Vision, PaLM 2
- **Cohere**: Command, Command Light, Command Nightly
- **Local Models**: Ollama integration and custom API endpoints

## 📦 Installation

### From Source
```bash
git clone https://github.com/njfio/fluent_cli.git
cd fluent_cli
cargo build --release
```

### Using Cargo
```bash
cargo install fluent-cli
```

## 🚀 Quick Start

### 1. Configure API Keys
```bash
# Set your preferred LLM provider API key
export OPENAI_API_KEY="your-api-key-here"
# or
export ANTHROPIC_API_KEY="your-api-key-here"
```

### 2. Basic LLM Interaction
```bash
# Simple query
fluent openai query "Explain quantum computing"

# Interactive chat
fluent openai chat

# Vision analysis
fluent openai vision image.jpg "What do you see?"
```

### 3. Agentic Mode
```bash
# Run an autonomous agent
fluent openai agent \
  --config agent_config.yaml \
  --goal "Analyze this codebase and suggest improvements" \
  --max-iterations 10 \
  --enable-tools

# Agent with MCP capabilities
fluent openai agent-mcp \
  --task "Read the README.md and create a project summary" \
  --servers "filesystem:mcp-server-filesystem,git:mcp-server-git"
```

### 4. MCP Server Mode
```bash
# Start as MCP server for other tools to use
fluent openai mcp --stdio
```

## 🤖 Agentic Capabilities

### Autonomous Agent Execution
```bash
# Complex task execution with learning
fluent openai agent \
  --config configs/coding_agent.yaml \
  --goal "Refactor the authentication system to use JWT tokens" \
  --enable-tools \
  --memory-threshold 0.8 \
  --max-iterations 20
```

### Agent Configuration Example
```yaml
# agent_config.yaml
agent:
  name: "fluent-coding-agent"
  description: "Advanced coding assistant with tool access"
  max_iterations: 50
  memory_threshold: 0.7
  reasoning_type: "chain_of_thought"
  
  tools:
    - file_operations
    - code_analysis
    - command_execution
    - web_search
  
  memory:
    type: "sqlite"
    path: "agent_memory.db"
    importance_threshold: 0.6
  
  learning:
    enabled: true
    adaptation_rate: 0.1
    pattern_recognition: true
```

### MCP Integration
```bash
# Connect to multiple MCP servers and execute intelligent tasks
fluent openai agent-mcp \
  --engine openai \
  --task "Analyze the git history and identify potential security issues" \
  --servers "git:mcp-server-git,security:mcp-server-security-scanner" \
  --config config.yaml
```

## 🔧 Advanced Configuration

### Multi-Engine Configuration
```yaml
# config.yaml
engines:
  - name: "openai-gpt4"
    engine: "openai"
    model: "gpt-4-turbo-preview"
    api_key: "${OPENAI_API_KEY}"
    max_tokens: 4000
    temperature: 0.7
    
  - name: "claude-opus"
    engine: "anthropic"
    model: "claude-3-opus-20240229"
    api_key: "${ANTHROPIC_API_KEY}"
    max_tokens: 4000
    temperature: 0.5
    
  - name: "gemini-pro"
    engine: "google"
    model: "gemini-pro"
    api_key: "${GOOGLE_API_KEY}"
    max_tokens: 2048
    temperature: 0.8

# Agent-specific configurations
agents:
  coding_assistant:
    engine: "openai-gpt4"
    tools: ["file_ops", "code_analysis", "git_ops"]
    memory_size: 1000
    learning_rate: 0.1
    
  research_agent:
    engine: "claude-opus"
    tools: ["web_search", "document_analysis", "summarization"]
    memory_size: 2000
    learning_rate: 0.05
```

## 🛠️ Tool System

### Built-in Tools
```bash
# File system operations
fluent openai agent --goal "Organize project files by type" --tools file_operations

# Code analysis and refactoring
fluent openai agent --goal "Add error handling to all functions" --tools code_analysis

# Command execution
fluent openai agent --goal "Set up CI/CD pipeline" --tools command_execution

# Web research
fluent openai agent --goal "Research latest Rust async patterns" --tools web_search
```

### MCP Tool Integration
```bash
# List available MCP tools
fluent openai agent-mcp --task "list available tools" --servers "filesystem:mcp-server-filesystem"

# Use specific MCP tools
fluent openai agent-mcp \
  --task "Use the git tools to create a feature branch and commit changes" \
  --servers "git:mcp-server-git"
```

## 🧠 Memory and Learning

### Persistent Agent Memory
```bash
# Agents automatically store and retrieve memories
fluent openai agent \
  --goal "Continue working on the user authentication feature" \
  --memory-db "project_memory.db"

# Query agent memories
fluent memory query "authentication implementation" --db "project_memory.db"

# Export learning insights
fluent memory export --format json --db "project_memory.db"
```

### Memory Types
- **Experience**: Records of successful task completions
- **Learning**: Insights gained from successes and failures
- **Strategy**: Effective approaches for different types of problems
- **Pattern**: Recognized patterns in code, data, or behavior
- **Rule**: Learned rules and constraints
- **Fact**: Important factual information

## 🌐 Model Context Protocol (MCP)

### As MCP Client
```bash
# Connect to filesystem MCP server
fluent openai agent-mcp \
  --task "Read all Python files and generate documentation" \
  --servers "filesystem:mcp-server-filesystem"

# Multi-server workflow
fluent openai agent-mcp \
  --task "Analyze git history, read code files, and generate a security report" \
  --servers "git:mcp-server-git,filesystem:mcp-server-filesystem,security:mcp-security-tools"
```

### As MCP Server
```bash
# Expose Fluent CLI capabilities via MCP
fluent openai mcp --stdio

# Use from VS Code or other MCP clients
# The server exposes Fluent CLI's agent capabilities as MCP tools
```

### MCP Server Configuration
```yaml
mcp:
  servers:
    filesystem:
      command: "mcp-server-filesystem"
      args: ["--stdio"]
    git:
      command: "mcp-server-git"
      args: ["--stdio"]
    custom:
      command: "python"
      args: ["custom_mcp_server.py", "--stdio"]
```

## 📊 Advanced Features

### Batch Processing
```bash
# Process multiple files with agents
fluent openai agent-batch \
  --goal "Add type hints to all Python functions" \
  --pattern "src/**/*.py" \
  --tools code_analysis

# Batch MCP operations
fluent openai agent-mcp-batch \
  --task "Generate README for each subdirectory" \
  --pattern "*/" \
  --servers "filesystem:mcp-server-filesystem"
```

### Workflow Orchestration
```bash
# Complex multi-step workflows
fluent openai workflow run \
  --file workflows/code_review.yaml \
  --input "src/" \
  --output "reports/"
```

### Real-time Monitoring
```bash
# Monitor agent execution
fluent openai agent \
  --goal "Implement new feature" \
  --monitor \
  --log-level debug

# Stream agent thoughts and actions
fluent openai agent \
  --goal "Debug performance issue" \
  --stream-thoughts \
  --tools profiling
```

## 🔍 Debugging and Introspection

### Agent State Inspection
```bash
# View agent's current state
fluent agent status --id agent_123

# Inspect agent memory
fluent agent memory --id agent_123 --query "recent learnings"

# View agent's reasoning process
fluent agent reasoning --id agent_123 --last 10
```

### Performance Monitoring
```bash
# Agent performance metrics
fluent agent metrics --id agent_123

# Tool usage statistics
fluent tools stats --agent agent_123

# Memory usage analysis
fluent memory analyze --db agent_memory.db
```

## 🧪 Development and Testing

### Building from Source
```bash
git clone https://github.com/njfio/fluent_cli.git
cd fluent_cli
cargo build --release
```

### Running Tests
```bash
# All tests
cargo test

# Agent-specific tests
cargo test --package fluent-agent

# MCP integration tests
cargo test mcp

# Integration tests with real LLMs (requires API keys)
cargo test --features integration_tests
```

### Development Mode
```bash
# Run with debug logging
RUST_LOG=debug fluent openai agent --goal "test task"

# Development configuration
fluent --config dev_config.yaml openai agent --goal "development task"
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup
```bash
git clone https://github.com/njfio/fluent_cli.git
cd fluent_cli
cargo build
cargo test
```

### Adding New Features
1. Fork the repository
2. Create a feature branch
3. Implement your feature with tests
4. Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **GitHub Issues**: [Report bugs or request features](https://github.com/njfio/fluent_cli/issues)
- **Discussions**: [Community discussions](https://github.com/njfio/fluent_cli/discussions)
- **Documentation**: [Full documentation](https://github.com/njfio/fluent_cli/wiki)
- **Examples**: [Usage examples](https://github.com/njfio/fluent_cli/tree/main/examples)

## 🎯 Roadmap

### Near Term
- [ ] Enhanced MCP protocol support (HTTP transport, resource subscriptions)
- [ ] Advanced tool composition and chaining
- [ ] Performance optimization for high-throughput scenarios
- [ ] Enhanced security and sandboxing features

### Medium Term
- [ ] Visual agent workflow designer
- [ ] Multi-agent coordination and collaboration
- [ ] Advanced learning algorithms and adaptation
- [ ] Cloud-based agent execution platform

### Long Term
- [ ] Natural language agent programming
- [ ] Autonomous agent marketplaces
- [ ] Integration with major development platforms
- [ ] Advanced AI reasoning and planning capabilities

---

**Fluent CLI: Where AI meets Development Excellence** 🚀
