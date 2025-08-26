# Repository Guidelines

## Project Structure & Module Organization
- `crates/`: Workspace crates (CLI, core, engines, storage, SDK, agent, lambda). Primary CLI logic lives in `crates/fluent-cli/`.
- `src/`: Root binary that delegates to the CLI (`main.rs`).
- `tests/`: End-to-end and integration tests (e.g., `e2e_cli_tests.rs`).
- `example_pipelines/`: Ready-to-run YAML pipelines (e.g., `test_pipeline.yaml`).
- `examples/`, `docs/`, `scripts/`: Additional samples, docs, and helper scripts.

## Build, Test, and Development Commands
- Build: `cargo build` (workspace build). Release: `cargo build --release`.
- Run CLI: `cargo run -- pipeline -f example_pipelines/test_pipeline.yaml -i "Hello"`
  - Global config: `--config fluent_config.toml` (default if present).
- Tests (all): `cargo test`  • CLI only: `cargo test -p fluent-cli`
- Lint/format: `cargo fmt --all` • `cargo clippy --all-targets -- -D warnings`
- Pre-commit: `pre-commit install && pre-commit run -a`

## Coding Style & Naming Conventions
- Rust 2021; format with `rustfmt`; lint with `clippy` (both enforced via pre-commit).
- Indentation: 4 spaces; line width: rustfmt defaults.
- Naming: `snake_case` for modules/functions, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Prefer explicit error types; map CLI errors to `CliError` for consistent exit codes.

## Testing Guidelines
- Framework: Rust test harness. Place E2E tests in `tests/` and crate-level tests in `crates/*/tests/`.
- Run a specific test: `cargo test --test e2e_cli_tests` or `cargo test <name>`.
- Keep tests deterministic; avoid network unless mocked. Use sample data under `tests/data/`.

## Commit & Pull Request Guidelines
- Commit style: Conventional Commits (e.g., `feat(cli): ...`, `fix(security): ...`, `chore:`).
- PRs must: describe changes, link issues, note breaking changes, include tests/docs, and pass fmt/clippy/tests.
- Add screenshots or sample CLI output for user-facing changes.

## Security & Configuration Tips
- Default config path is `fluent_config.toml`. Do not commit secrets; prefer environment variables or untracked config files.
- Errors are redacted; still avoid logging sensitive data. Validate external inputs and handle network failures explicitly.

