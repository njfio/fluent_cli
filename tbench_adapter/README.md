# Fluent CLI Terminal-Bench Adapter

This adapter allows you to run the Fluent CLI agent within the [Terminal-Bench](https://tbench.ai) evaluation harness.

## Prerequisites

1. Install Terminal-Bench:
   ```bash
   uv tool install terminal-bench
   ```

2. Ensure Docker is running (Terminal-Bench uses Docker containers)

3. Set API keys in your environment:
   ```bash
   export ANTHROPIC_API_KEY=your_key_here
   # Or for OpenAI models:
   export OPENAI_API_KEY=your_key_here
   ```

## Quick Start

### Option 1: Build from Source in Container (Slower, Always Works)

Run the adapter without any pre-built binary. The installation script will compile Fluent CLI from source inside the container:

```bash
cd /path/to/fluent_cli
PYTHONPATH="${PYTHONPATH}:$(pwd)" tb run \
  --agent-import-path tbench_adapter.fluent_agent:FluentAgent \
  -d terminal-bench-core \
  --n-tasks 1
```

Note: Building from source takes 5-10 minutes on first run due to Rust compilation.

### Option 2: Pre-built Binary (Faster)

For faster execution, build a Linux binary and mount it:

1. Cross-compile for Linux (from macOS):
   ```bash
   # Install cross-compilation toolchain
   rustup target add aarch64-unknown-linux-gnu
   # Or for x86_64:
   rustup target add x86_64-unknown-linux-gnu

   # Build
   cargo build --release -p fluent-cli --target aarch64-unknown-linux-gnu

   # Copy to mount directory
   mkdir -p .fluent_binary
   cp target/aarch64-unknown-linux-gnu/release/fluent .fluent_binary/
   ```

2. The install script will automatically detect and use the binary from `/workspace/.fluent_binary/fluent`.

## Agent Variants

The adapter provides three agent variants:

### FluentAgent (Default)
Standard configuration with 50 max iterations.

```bash
tb run --agent-import-path tbench_adapter.fluent_agent:FluentAgent -d terminal-bench-core
```

### FluentAgentReflection
Enables reflection mode for more thoughtful reasoning.

```bash
tb run --agent-import-path tbench_adapter.fluent_agent:FluentAgentReflection -d terminal-bench-core
```

### FluentAgentFast
Configured for faster iteration with 20 max iterations (useful for simple tasks).

```bash
tb run --agent-import-path tbench_adapter.fluent_agent:FluentAgentFast -d terminal-bench-core
```

## Configuration

### Agent Constructor Arguments

Pass custom arguments using `--agent-kwarg`:

```bash
tb run \
  --agent-import-path tbench_adapter.fluent_agent:FluentAgent \
  --agent-kwarg model=claude-3-5-sonnet-20241022 \
  --agent-kwarg max_iterations=100 \
  -d terminal-bench-core
```

Available kwargs:
- `model`: LLM model to use (default: `claude-sonnet-4-20250514`)
- `max_iterations`: Maximum agent iterations (default: `50`)
- `enable_reflection`: Enable reflection mode (default: `false`)

### Environment Variables

Set in your shell before running:

- `ANTHROPIC_API_KEY`: Required for Anthropic models
- `OPENAI_API_KEY`: Required for OpenAI models
- `GOOGLE_API_KEY`: Required for Google models
- `FLUENT_MODEL`: Override the default model
- `FLUENT_MAX_ITERATIONS`: Override max iterations

## Example Commands

Run a single task:
```bash
PYTHONPATH="${PYTHONPATH}:$(pwd)" tb run \
  --agent-import-path tbench_adapter.fluent_agent:FluentAgent \
  -d terminal-bench-core \
  --n-tasks 1 \
  --livestream
```

Run specific task by ID:
```bash
PYTHONPATH="${PYTHONPATH}:$(pwd)" tb run \
  --agent-import-path tbench_adapter.fluent_agent:FluentAgent \
  -d terminal-bench-core \
  -t hello-world
```

Run with multiple concurrent tasks:
```bash
PYTHONPATH="${PYTHONPATH}:$(pwd)" tb run \
  --agent-import-path tbench_adapter.fluent_agent:FluentAgent \
  -d terminal-bench-core \
  --n-concurrent 4 \
  --n-tasks 10
```

## Output

Results are saved to `runs/<timestamp>/` including:
- `run.log`: Full execution log
- `results.json`: Task results and scores
- `<task-id>/`: Per-task outputs and recordings

## Troubleshooting

### "No pre-built binary found, building from source..."
This is expected if you haven't provided a pre-built Linux binary. The build process will take a few minutes.

### Container installation fails
Ensure Docker has sufficient memory allocated (at least 4GB recommended for compilation).

### API key errors
Make sure your API keys are set in your environment before running `tb run`.
