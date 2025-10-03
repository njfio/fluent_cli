# Fluent CLI

Fluent CLI is a Rust workspace that provides a modular command-line interface for orchestrating Large Language Model (LLM) workflows. The CLI layers configuration management, a pipeline runner, goal-directed agent mode, tool execution, and Model Context Protocol (MCP) utilities on top of shared engine adapters in `crates/fluent-engines` and agent infrastructure in `crates/fluent-agent`.

The repository now focuses on the production code paths only. All generated documentation, examples, and ad-hoc research artefacts were removed, so this README is the primary source of project information.

## Feature Overview
- **Multi-provider engine abstraction** – `fluent-engines` implements adapters for OpenAI, Anthropic, Google Gemini, Cohere, Mistral, Groq, Perplexity, StabilityAI, Langflow/Flowise, webhooks, and related providers. Engines are selected through the workspace configuration layer in `fluent-core`.
- **Pipeline executor** – `fluent-engines::pipeline_executor` runs YAML-defined pipelines with optional state persistence and resumable execution.
- **Agent mode** – `fluent-cli` exposes an interactive and non-interactive agent loop (`fluent agent --goal …`) that drives the higher level reasoning/orchestration code from `crates/fluent-agent`. An optional TUI is available via `--tui`.
- **Tooling system** – The agent toolbox bundles safe filesystem access, a guarded shell runner, a string-replace editor, and a Rust build/test helper. All tools enforce whitelists and size limits defined in `ToolExecutionConfig`.
- **MCP utilities** – `fluent mcp …` bridges the Model Context Protocol server/client implementation in `crates/fluent-agent`.
- **Neo4j helpers** – `fluent neo4j …` surfaces the Cypher generation utilities in `fluent-cli::neo4j_runner`.
- **Shell completions** – `fluent completions --shell <bash|zsh|fish|powershell|elvish>` emits completion scripts built from the current CLI definition.

## Getting Started
### Prerequisites
- Rust 1.79+ and Cargo.
- API keys for any engines you intend to call, supplied via environment variables (for example `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY`).

### Build
```bash
cargo build --release
```

### Configure Engines
`fluent_config.toml` is the default workspace configuration. Populate it with the engines you need; a minimal Anthropic example is shipped in the repository:

```toml
[[engines]]
name = "anthropic"
engine = "anthropic"

[engines.connection]
protocol = "https"
hostname = "api.anthropic.com"
port = 443
request_path = "/v1/messages"

[engines.parameters]
bearer_token = "${ANTHROPIC_API_KEY}"
modelName = "claude-3-7-sonnet-20250219"
temperature = 0.1
max_tokens = 4000
system = "You are an expert Rust programmer and game developer."
```

Additional sample JSON configurations (for example `anthropic_config.json`) mirror the same schema and can be copied or adapted.

### Running the CLI
```bash
# List configured engines
cargo run -- engine list

# Probe engine connectivity
cargo run -- engine test anthropic

# Execute a pipeline (author pipeline.yaml first)
cargo run -- pipeline -f pipeline.yaml -i "Write a summary of docs"

# Launch an agentic run
cargo run -- agent --goal "Produce a refactoring plan for src/lib.rs" --enable-tools

# Start the MCP server over stdio
cargo run -- mcp server --stdio

# Inspect available tools
cargo run -- tools list
```

Pipelines are defined with the `Pipeline` schema in `fluent_engines::pipeline_executor`. A minimal example that echoes the provided input:

```yaml
name: "echo"
steps:
  - PrintOutput:
      name: "show-input"
      value: "Pipeline input: ${input}"
  - Command:
      name: "echo-input"
      command: "echo ${input}"
      save_output: "echo_result"
  - PrintOutput:
      name: "show-result"
      value: "Echoed output: ${echo_result}"
```

## Agent Mode
The `agent` subcommand provides both interactive and goal-driven flows.

- Non-interactive goals:
  ```bash
  cargo run -- agent --goal "Add logging to pipeline executor" --max-iterations 12 --enable-tools
  ```
- Interactive REPL (requires a TTY):
  ```bash
  cargo run -- agent
  ```
- Optional terminal UI:
  ```bash
  cargo run -- agent --goal "Investigate memory usage" --tui
  ```

Agent tools are disabled by default unless `--enable-tools` or the agent config explicitly enables them.

## Security Posture
Security-sensitive functionality is gated and validated throughout the codebase:
- **Tool whitelists and sanitisation** – `ShellExecutor` and the command validation helpers in `crates/fluent-agent::tools` reject commands containing dangerous constructs (`&&`, pipelines, path traversal, interpreters, etc.) and only allow the prefixes configured in `ToolExecutionConfig.allowed_commands`.
- **Path confinement** – File operations and the string replace editor call `validation::validate_path`, ensuring targets stay within the allowed directories declared in tool configuration.
- **Runtime feature flags** – Potentially risky behaviours in `fluent-core::output_processor` (ad-hoc script execution and shell commands) are switched off unless the operator sets `FLUENT_ENABLE_SCRIPT_EXECUTION=true` or `FLUENT_ENABLE_COMMAND_EXECUTION=true`.
- **Environment hygiene** – Shell commands launched from the output processor use `env_clear()` with a minimal `PATH`, and agent shell tools drop stdin to prevent interactive escalation.
- **Secrets** – No API keys or credentials are committed; sample configurations reference environment substitutions.

During the review no hardcoded credentials or obvious injection paths were found. If you extend the tool whitelist or enable script execution, audit those changes carefully.

## Development
```bash
# Workspace checks
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

Most crates retain their unit tests; integration fixtures and generated documentation were intentionally removed. Add new tests near the code they cover.

## Project Layout
```
fluent_cli/
├── crates/
│   ├── fluent-cli/      # CLI surface, commands, TUI helpers, CLI builder
│   ├── fluent-agent/    # Agent runtime, tool system, MCP support
│   ├── fluent-core/     # Shared config layer, error types, utilities
│   ├── fluent-engines/  # Engine adapters and pipeline executor
│   ├── fluent-storage/  # Persistence helpers
│   └── fluent-sdk/      # SDK utilities for embedding Fluent in other apps
├── scripts/             # Security and quality helper scripts
├── fluent_config.toml   # Default engine configuration template
├── AGENTS.md / CLAUDE.md / WARP.md  # High-level notes retained after cleanup
└── LICENSE              # Apache-2.0
```

## License
Apache License 2.0 – see [`LICENSE`](LICENSE).
