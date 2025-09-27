# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build Commands
```bash
# Build the entire workspace
cargo build

# Build release version (optimized)
cargo build --release

# Build specific crate
cargo build -p fluent-cli
```

### Test Commands
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fluent-cli
cargo test -p fluent-agent

# Run integration tests
cargo test --test integration
cargo test --test e2e_cli_tests

# Run specific test
cargo test test_name

# Run with output displayed
cargo test -- --nocapture
```

### Lint and Format Commands
```bash
# Format all code
cargo fmt --all

# Check formatting without applying
cargo fmt --all -- --check

# Run clippy (linter) with strict warnings
cargo clippy --all-targets -- -D warnings

# Run pre-commit hooks (if installed)
pre-commit run -a
```

### Running the CLI
```bash
# Basic CLI execution
cargo run -- <command>

# Run with pipeline
cargo run -- pipeline -f example_pipelines/test_pipeline.yaml -i "Hello"

# With custom config
cargo run -- --config fluent_config.toml <command>

# Direct engine query
cargo run -- <engine-name> "Your prompt here"

# Agent mode
cargo run -- agent
```

## Architecture

### Workspace Structure
The project uses a Cargo workspace with multiple crates providing modular functionality:

- **fluent-cli**: Main CLI application handling command parsing, orchestration, and user interaction. Contains modular command handlers (`commands/` module) for agent, pipeline, MCP, Neo4j, engine, and tools operations.

- **fluent-agent**: Advanced agentic framework providing autonomous capabilities. Implements ReAct loop, reasoning engines, planning systems, memory management, reflection engine, and MCP integration. Production-ready with comprehensive security controls.

- **fluent-core**: Shared utilities, configuration management, traits, and types. Provides base abstractions like `Engine` trait, `Request`/`Response` types, error handling, Neo4j client, and centralized configuration.

- **fluent-engines**: Multi-provider LLM implementations (OpenAI, Anthropic, Google, Cohere, Mistral, etc.). Includes pipeline executor, streaming support, connection pooling, caching, and plugin system.

- **fluent-storage**: Persistent storage layer with vector database support, embeddings, and memory storage backends.

- **fluent-sdk**: SDK for external integrations and library usage.

- **fluent-config**: Configuration management binary with schema generation and validation.

### Key Design Patterns

1. **Trait-Based Engine System**: All LLM providers implement the `Engine` trait from fluent-core, allowing uniform interface across different providers.

2. **Async-First Architecture**: Extensive use of Tokio for async operations, particularly in engine implementations and agent systems.

3. **Security-By-Default**: Command validation, path restrictions, and input sanitization built into the agent framework. Security framework in `fluent-agent/src/security/`.

4. **Modular Command Structure**: CLI commands are organized as separate modules under `fluent-cli/src/commands/`, each handling specific functionality domains.

5. **MCP Integration**: Model Context Protocol support through both client and server implementations in fluent-agent, enabling tool integration and inter-process communication.

### Configuration System

The application uses a hierarchical configuration system:
- Global config via `fluent_config.toml` or `--config` flag
- Engine configurations in YAML format defining LLM provider settings
- Pipeline definitions in YAML for multi-step workflows
- Agent configurations for autonomous behavior settings
- Environment variables for API keys and sensitive data

### Memory and State Management

The agent system includes sophisticated memory management:
- SQLite-based persistent memory in `fluent-agent/src/memory/`
- Working memory for immediate context
- Cross-session persistence for long-term learning
- Context compression for efficient storage
- State checkpointing and restoration

### Tool System

Comprehensive tool framework in `fluent-agent/src/tools/`:
- File operations (read, write, list, create directories)
- String replace editor for surgical file modifications
- Shell command execution with security controls
- Rust compiler integration (cargo commands)
- Workflow composition tools

### Testing Infrastructure

- Unit tests alongside implementation files
- Integration tests in `tests/` directory
- E2E tests in `tests/e2e_cli_tests.rs`
- Functional tests in `tests/functional_tests/`
- Example demonstrations in `examples/`
- Test data fixtures in `tests/data/`

## Important Notes

1. **API Keys**: Always use environment variables for API keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.). Never commit credentials.

2. **Error Handling**: The codebase uses comprehensive Result types. Production code has zero unwrap() calls in critical paths.

3. **Security**: Command execution goes through validation. See `FLUENT_ALLOW_COMMANDS` and `FLUENT_DISALLOW_COMMANDS` environment variables for runtime configuration.

4. **Logging**: Supports both human-readable and JSON logging. Set `FLUENT_LOG_FORMAT=json` or use `--json-logs` flag.

5. **Feature Flags**: Some experimental features may be behind feature flags in Cargo.toml files.

6. **Workspace Dependencies**: Dependencies are managed at workspace level in root Cargo.toml for consistency.