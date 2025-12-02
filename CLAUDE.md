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
cargo test -p fluent-core

# Run integration tests
cargo test --test integration
cargo test --test e2e_cli_tests
cargo test --test json_output_tests
cargo test --test exit_code_tests

# Run functional tests (subset)
cargo test --test functional_tests

# Run specific test by name
cargo test test_name

# Run with output displayed
cargo test -- --nocapture

# Run tests with specific pattern
cargo test reflection -- --nocapture
cargo test security -- --nocapture
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

# Generate shell completions
cargo run -- completions --shell zsh > _fluent
cargo run -- completions --shell bash > fluent.bash
cargo run -- completions --shell fish > fluent.fish

# Print config schema (JSON Schema format)
cargo run -- schema

# Tools commands
cargo run -- tools list
cargo run -- tools describe <tool-name>
cargo run -- tools exec <tool-name> [args]

# Engine management
cargo run -- engine list
cargo run -- engine test <engine-name>
```

## Architecture

### Workspace Structure
The project uses a Cargo workspace with multiple crates providing modular functionality:

- **fluent-cli**: Main CLI application handling command parsing, orchestration, and user interaction. Contains modular command handlers (`commands/` module) for agent, pipeline, MCP, Neo4j, engine, and tools operations.

- **fluent-agent**: Advanced agentic framework providing autonomous capabilities. Implements ReAct loop, reasoning engines, planning systems, memory management, reflection engine, and MCP integration. Production-ready with comprehensive security controls.

- **fluent-core**: Shared utilities, configuration management, traits, and types. Provides base abstractions like `Engine` trait, `Request`/`Response` types, error handling, Neo4j client, and centralized configuration.

- **fluent-engines**: Multi-provider LLM implementations (OpenAI, Anthropic, Google, Cohere, Mistral, etc.). Includes pipeline executor, streaming support, connection pooling, and caching. **Note**: Plugin system code exists but is disabled (see Plugin System section below).

- **fluent-storage**: Persistent storage layer with vector database support, embeddings, and memory storage backends.

- **fluent-sdk**: SDK for external integrations and library usage.

- **fluent-config**: Configuration management binary with schema generation and validation.

### Key Design Patterns

1. **Trait-Based Engine System**: All LLM providers implement the `Engine` trait from fluent-core, allowing uniform interface across different providers.

2. **Async-First Architecture**: Extensive use of Tokio for async operations, particularly in engine implementations and agent systems.

3. **Security-By-Default**: Command validation, path restrictions, and input sanitization built into the agent framework. Security framework in `fluent-agent/src/security/`.

4. **Modular Command Structure**: CLI commands are organized as separate modules under `fluent-cli/src/commands/`, each implementing the `CommandHandler` trait:
   - `agent.rs` - Agentic execution and interactive mode
   - `pipeline.rs` - Pipeline execution and building
   - `mcp.rs` - Model Context Protocol server/client
   - `neo4j.rs` - Neo4j graph database operations
   - `engine.rs` - Engine management and testing
   - `tools.rs` - Direct tool access and execution

5. **MCP Integration**: Model Context Protocol support through both client and server implementations in fluent-agent, enabling tool integration and inter-process communication.

6. **CommandHandler Pattern**: All commands implement a consistent `CommandHandler` trait with `async fn execute(&self, matches: &ArgMatches, config: &Config) -> Result<()>` for uniform command execution.

### Configuration System

The application uses a hierarchical configuration system:
- Global config via `fluent_config.toml` or `--config` flag
- Engine configurations in YAML format defining LLM provider settings
- Pipeline definitions in YAML for multi-step workflows
- Agent configurations for autonomous behavior settings
- Environment variables for API keys and sensitive data
- JSON Schema generation via `fluent-config` binary or `fluent schema` command

**Config-Optional Commands**: Some commands (like `tools`, `completions`, `engine list`) can run without a config file and will use minimal defaults.

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

### Plugin System Status

**IMPORTANT: The plugin system is DISABLED and not available in production builds.**

#### Why Plugins Are Disabled

The codebase contains a complete secure plugin architecture in `crates/fluent-engines/src/plugin.rs` and `secure_plugin_system.rs`, but it is intentionally disabled for the following reasons:

1. **WASM Runtime Not Included**
   - Requires wasmtime or wasmer (~10-15MB binary size increase)
   - `wasm-runtime` feature flag is disabled by default
   - WASM execution layer is not implemented (returns error)

2. **Security Infrastructure Requirements**
   - Requires PKI setup for Ed25519 signature verification
   - No trusted plugin registry or distribution mechanism
   - Needs comprehensive security audit before production use
   - Supply chain attack risks from untrusted plugins

3. **Maintenance and Support Burden**
   - Plugin API stability guarantees required
   - Ongoing security updates and patches needed
   - Support burden for third-party plugin developers

#### What's Implemented (But Disabled)

The secure plugin system includes:
- ✅ Complete plugin manifest system with capabilities and permissions
- ✅ Cryptographic signature verification (Ed25519)
- ✅ Resource limits and quotas (memory, CPU, network)
- ✅ Capability-based security model
- ✅ Comprehensive audit logging
- ✅ Plugin CLI management tool (`plugin_cli.rs`)
- ⚠️ WASM runtime execution (architecture ready, but not implemented)

#### Alternatives to Plugins

Instead of plugins, use:
1. **Built-in engines**: OpenAI, Anthropic, Google Gemini, Cohere, Mistral, Groq, Perplexity, StabilityAI, Leonardo AI, DALL-E
2. **Webhook engine**: Proxy requests to custom external services
3. **Fork and add**: Submit a PR to add your engine as a built-in type
4. **Langflow/Flowise**: Use these chain engines for custom workflows

#### Enabling for Development (Not Recommended)

If you need to enable plugins for development/testing:
1. Add WASM runtime to `crates/fluent-engines/Cargo.toml`
2. Implement WASM execution in `SecurePluginEngine::execute()`
3. Set up Ed25519 key infrastructure
4. Build with `cargo build --features wasm-runtime`

See detailed documentation in `crates/fluent-engines/src/plugin.rs` module docs.

## Important Notes

1. **API Keys**: Always use environment variables for API keys (OPENAI_API_KEY, ANTHROPIC_API_KEY, etc.). Never commit credentials.

2. **Error Handling**: The codebase uses comprehensive Result types. Production code has zero unwrap() calls in critical paths.

3. **Security**: Command execution goes through validation. See `FLUENT_ALLOW_COMMANDS` and `FLUENT_DISALLOW_COMMANDS` environment variables for runtime configuration.

4. **Logging**: Supports both human-readable and JSON logging via:
   - Environment variable: `FLUENT_LOG_FORMAT=json` or `FLUENT_LOG_FORMAT=human`
   - CLI flags: `--json-logs` or `--human-logs`
   - Verbosity: `--verbose` (sets `FLUENT_VERBOSE=1`) or `--quiet` (sets `FLUENT_QUIET=1`)
   - Tracing-based logging with request IDs for correlation

5. **Feature Flags**: Some experimental features may be behind feature flags in Cargo.toml files.

6. **Workspace Dependencies**: Dependencies are managed at workspace level in root Cargo.toml for consistency. Pin critical dependencies (reqwest, tokio, serde) to specific versions.

7. **Request IDs**: All operations generate unique request IDs for tracing and debugging. Look for `request_id` in JSON logs or structured output.

8. **Config Schema**: The `EnhancedEngineConfig` JSON Schema can be generated with `fluent schema` or via the `fluent-config` binary for validation and documentation.