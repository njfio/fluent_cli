# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

Repository: Fluent CLI (Rust workspace; multi-crate)

What this file covers
- Common commands to build, lint/format, and test (including running a single test and functional test suites)
- High-level architecture so future Warp sessions can get productive quickly
- Where to look for deeper architecture details and CLI usage

Common commands

- Build and run
  - Build (debug): cargo build
  - Build (release): cargo build --release
  - Run CLI (show help): cargo run -- --help
  - After building, the binary is target/release/fluent (root package name is "fluent")

- Tests
  - Run all tests (workspace): cargo test
  - Run integration/functional tests crate: cargo test -p fluent-integration-tests
  - Run a specific test target: cargo test -p fluent-integration-tests --test cli_functional_tests
  - Run a single test by name/pattern:
    - cargo test -p fluent-integration-tests --test cli_functional_tests <pattern>

- Examples (selected)
  - cargo run --example reflection_demo
  - cargo run --example state_management_demo
  - cargo run --example string_replace_demo

- Lint/format and pre-commit
  - Format (apply): cargo fmt --all
  - Format (check): cargo fmt --all -- --check
  - Lint (deny warnings): cargo clippy --all-targets --all-features -D warnings
  - Pre-commit hooks:
    - pre-commit install
    - pre-commit run -a

- Functional test suites and scripts
  - Full functional suite runner: ./tests/functional_tests/run_all_tests.sh
  - Shell-based command coverage: ./tests/functional_tests/test_all_cli_commands.sh
  - Python scenarios: ./tests/functional_tests/test_cli_scenarios.py

- Security and quality scripts
  - Security audit (15 checks): ./scripts/security_audit.sh
  - Code quality checks: ./scripts/code_quality_check.sh

- CLI notes
  - Top-level subcommands: pipeline, agent, mcp, neo4j, engine, tools
  - Help for a subcommand: cargo run -- tools --help (replace tools with any subcommand)
  - Tool listing and descriptions (no API keys required):
    - cargo run -- tools list
    - cargo run -- tools categories
    - cargo run -- tools describe read_file
  - Engine-backed requests require a config file and provider API keys (set via env vars) and are invoked through subcommands like engine, pipeline, or agent.

High-level architecture and structure

Workspace overview (big picture)
- Root binary: fluent (src/main.rs)
  - Initializes env_logger and delegates to fluent_cli::cli::run_modular()
  - Maps errors to consistent exit codes (e.g., 2=arg parse, 10=config, 12=network, 13=engine, etc.)
- Crates (major roles):
  - fluent-cli: CLI surface and orchestration
    - Command routing via cli.rs/cli_builder.rs to modular handlers in crates/fluent-cli/src/commands/:
      - agent (agentic workflows), pipeline (YAML pipelines), mcp (MCP server/client), neo4j, engine, tools
    - Additional modules: request/response formatting, validation, memory helpers, MCP/Neo4j runners
  - fluent-core: Core types/traits, configuration, auth, caching, error types, validation, Neo4j integration
  - fluent-engines: Provider integrations (OpenAI, Anthropic, Gemini, etc.), modular pipeline executors, connection pooling, caching, shared HTTP utilities
  - fluent-agent: Agentic system (ReAct loop: reasoning → planning → execution → observation → memory), tool registry (filesystem, compiler, shell, editor), memory systems, reflection, MCP integration, and security/sandboxing
  - fluent-storage: Persistence utilities (e.g., SQLite) and storage concerns
  - fluent-sdk: External integration surfaces (SDK for engine/config interactions)
  - fluent-lambda: AWS Lambda integration surface for serverless deployments
  - tests (crate): fluent-integration-tests provides organized E2E/functional coverage and runners

Key flows
- Direct LLM interaction: CLI → engine selection → provider API → response processing → output
- Pipeline execution: YAML pipeline → parsing/validation → step/parallel/condition/loop executors → result aggregation
- Agentic execution (ReAct): goal → reasoning → action planning → tool execution → observation → memory update → loop until complete
- MCP integration: MCP client ↔ MCP server (JSON-RPC) → tool registry → engine execution

Where to look for details
- README.md (root): Build/test/run instructions, feature overview, examples, and usage
- docs/architecture/
  - SYSTEM_ARCHITECTURE.md: Layered system design and core components
  - COMPONENT_ARCHITECTURE.md: Command system, agent orchestrator, engine traits, pipeline, memory, tools, MCP
  - DATA_FLOW_ARCHITECTURE.md: Request/response, pipeline, ReAct, MCP, config, errors, metrics
  - DEPLOYMENT_ARCHITECTURE.md: Local/desktop, server (MCP), container/K8s, Lambda
  - SECURITY_ARCHITECTURE.md: Threat model, validation, authN/Z, sandboxing, audit
- tests/functional_tests/README.md and COMPREHENSIVE_TESTING_GUIDE.md: Functional coverage layout and how to run suites
- .github/workflows/rust.yml: CI reference for fmt, clippy, and test invocations
- .pre-commit-config.yaml: Local hooks for rustfmt and clippy
- AGENTS.md: Repository structure, commands, and conventions for development

Notes and expectations
- Engine-backed operations require a correctly configured engine entry (see README config examples). Use --config to point to your file; the CLI defaults to fluent_config.toml if present.
- Many functional tests are designed to be non-destructive and run without network/API keys; scripts use dry-run and temporary directories.
- Exit codes are intentionally categorized and validated by tests (see tests/exit_code_tests.rs).
