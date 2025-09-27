# GEMINI.md

## Project Overview

This project is a Rust-based command-line interface (CLI) called "Fluent CLI". It provides a unified interface for interacting with multiple Large Language Model (LLM) providers, including OpenAI, Anthropic, and Google.

The project is structured as a Rust workspace with multiple crates, each responsible for a specific functionality:

*   `fluent-cli`: The main CLI application.
*   `fluent-core`: Core utilities and configuration.
*   `fluent-engines`: LLM engine implementations.
*   `fluent-agent`: Agentic capabilities and tools.
*   `fluent-storage`: Storage and persistence layer.
*   `fluent-sdk`: SDK for external integrations.

Fluent CLI has advanced features such as:

*   **Agentic Capabilities:** An experimental agentic system with a ReAct loop, a tool system for file operations, shell commands, and code analysis.
*   **Pipeline Execution:** A YAML-based pipeline system for defining and executing multi-step workflows.
*   **Model Context Protocol (MCP):** Integration with MCP for tool integration.
*   **Configuration Management:** YAML-based configuration for multiple LLM engines.
*   **Self-Reflection and Learning:** An experimental system for self-reflection and learning.

## Building and Running

### Building the Project

To build the project, run the following command:

```bash
cargo build --release
```

### Running the CLI

The main executable is `fluent`. You can use it with various subcommands:

*   **Direct LLM Queries:**

    ```bash
    fluent openai-gpt4 "Explain quantum computing"
    ```

*   **Agent Commands:**

    ```bash
    fluent agent
    ```

*   **Pipeline Commands:**

    ```bash
    fluent pipeline -f pipeline.yaml -i "process this data"
    ```

*   **MCP (Model Context Protocol) Commands:**

    ```bash
    fluent mcp server --stdio
    ```

### Running Tests

To run the test suite, use the following command:

```bash
cargo test
```

## Development Conventions

*   **Modular Architecture:** The project follows a modular architecture with functionalities separated into different crates.
*   **Error Handling:** The project has a comprehensive error handling system using the `anyhow` and `thiserror` crates.
*   **Logging:** The project uses the `tracing` and `env_logger` crates for logging.
*   **Configuration:** The application is configured using YAML files (`config.yaml`, `fluent_config.toml`).
*   **Code Quality:** The project has scripts for security audits (`scripts/security_audit.sh`) and code quality checks (`scripts/code_quality_check.sh`).
*   **Dependencies:** The project uses a wide range of dependencies, including `tokio` for asynchronous programming, `clap` for command-line argument parsing, `serde` for serialization/deserialization, and `reqwest` for making HTTP requests.
