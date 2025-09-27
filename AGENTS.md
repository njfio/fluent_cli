# Repository Guidelines

## Project Structure & Module Organization
- `crates/` hosts the workspace crates; CLI behaviour lives in `crates/fluent-cli/src/` alongside shared engine, storage, and SDK layers.
- `src/main.rs` only bootstraps the CLI; keep new logic inside the relevant crate.
- `tests/` carries integration and E2E coverage, including `e2e_cli_tests.rs` and fixtures under `tests/data/`.
- `example_pipelines/` and `example_configurations/` expose runnable YAML scenarios you can use for local validation.
- Support material sits in `examples/`, `docs/`, and `scripts/`; add new tooling there to keep the root clean.

## Build, Test, and Development Commands
- `cargo build` compiles the entire workspace; add `--release` for optimized binaries.
- `cargo run -- pipeline -f example_pipelines/test_pipeline.yaml -i "Hello"` drives the CLI against a sample pipeline. Pass `--config fluent_config.toml` to override defaults.
- `cargo test` executes all unit and integration suites; scope with `-p fluent-cli` for CLI-only checks.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` gate formatting and linting; run them before proposing changes.
- `pre-commit install && pre-commit run -a` mirrors CI hooks locally.

## Coding Style & Naming Conventions
- Follow Rust 2021 defaults: 4-space indentation, rustfmt line widths, and module organization guidelines.
- Name files and functions with `snake_case`, types with `CamelCase`, and consts with `SCREAMING_SNAKE_CASE`.
- Prefer explicit error types and map user-facing failures to `CliError` for consistent exit codes.

## Testing Guidelines
- Use the Rust test harness and keep specs deterministic; avoid network calls unless mocked.
- Place crate-specific tests under `crates/<name>/tests/` and broader scenarios in `tests/`.
- Reference shared fixtures in `tests/data/`, or add new ones there when extending coverage.

## Commit & Pull Request Guidelines
- Write Conventional Commits such as `feat(cli): add pipeline flag` or `fix(security): guard config loading`.
- PRs should summarise intent, link issues, flag breaking changes, and include screenshots or CLI output for UX updates.
- Ensure `cargo fmt`, `cargo clippy`, and `cargo test` succeed locally before requesting review.

## Security & Configuration Tips
- Default to `fluent_config.toml`; never commit secrets or tokens, and prefer environment variables for overrides.
- Validate external inputs, handle expected network failures explicitly, and rely on redacted logging to avoid leaking data.
