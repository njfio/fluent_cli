# Fluent CLI - Multi-LLM Command Line Interface

A Rust-based command-line interface for interacting with multiple Large Language Model (LLM) providers. Fluent CLI provides a unified interface for OpenAI, Anthropic, Google Gemini, and other LLM services, with experimental agentic capabilities and Model Context Protocol (MCP) integration.

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

### 🤖 **Experimental Agentic Features**
- **Basic Agent Loop**: Interactive agent sessions with memory
- **MCP Integration**: Model Context Protocol client and server capabilities
- **Tool System**: File operations, shell commands, and code analysis
- **Memory System**: SQLite-based persistent memory for agents

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

### 3. Available Commands
```bash
# Interactive agent session
fluent openai agent

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

## 🤖 Experimental Features

### Agent Mode
Interactive agent sessions with basic memory and tool access:

```bash
# Start an interactive agent session
fluent openai agent

# Agent with specific goal (experimental)
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

### Experimental Features
- **Agent System**: 🚧 Basic implementation, under development
- **MCP Integration**: 🚧 Prototype stage
- **Tool System**: 🚧 Limited functionality
- **Memory System**: 🚧 Basic SQLite storage

### Planned Features
- Enhanced agent capabilities
- Expanded tool ecosystem
- Advanced MCP client/server features
- Improved memory and learning systems

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
