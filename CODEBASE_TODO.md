Fluent CLI Codebase — Comprehensive TODO Plan

Scope
- This plan covers the entire repository (workspace crates, CLI, engines, agent, SDK, storage, lambda, examples, tests, and docs).
- Each item is phrased as an actionable task with suggested ownership and references where helpful.

Priority Key
- [P0] Blockers or correctness/security issues
- [P1] High impact improvements and reliability
- [P2] Quality, DX, docs, and nice-to-haves


Immediate and Correctness
- [P0] [DONE] Resolve deprecated SqliteMemoryStore usage by completing AsyncSqliteMemoryStore LongTermMemory trait
  - crates/fluent-agent/src/memory.rs
  - Implement trait with proper async lifetimes; migrate call sites and examples; remove deprecation warnings
  - Update TECHNICAL_DEBT.md after completion
- [P0] Replace ad‑hoc error fixer utility with a robust diagnostics pipeline
  - src/error_fixer.rs: remove hardcoded line numbers and bespoke string matching
  - Introduce rustc/json-diagnostics parsing (RUSTFLAGS='-Zunstable-options' or use cargo check --message-format=json) and apply structured fixes or suggestions
  - If the feature is experimental, move it under examples/ or behind a feature flag
- [P1] [DONE] Ensure the CLI runs with no required config present (graceful defaults already exist) and add test coverage for that path
  - crates/fluent-cli/src/cli.rs: expand tests to assert no panic and meaningful help when no config is found
- [P1] Validate all direct Engine invocations through CLI (back-compat paths) still behave correctly with missing API keys
  - Add tests asserting clear user-facing errors and non-zero exit codes without panics
- [P1] Harden command execution tool security
  - crates/fluent-agent/src/lib.rs run_command: review allowed command list and pattern filters
  - Add allowlist categories by context; enforce timeouts and maximum output size
  - Add tests for deny-by-default behavior and for bad arguments with metacharacters
- [P1] Normalize create_engine usage and error surfaces
  - [DONE] crates/fluent-engines/src/lib.rs create_engine: ensure unknown engines produce uniform, user-friendly errors; add tests
- [P1] Confirm feature flags and examples compile without hidden deps
  - Ensure examples that depend on API keys skip at runtime with clear messages if keys are absent


Security and Hardening
- [P0] Secrets handling and propagation
  - Audit all providers for bearer_token/env usage; ensure no defaults embed secrets; ensure error messages never echo secret values
  - Add an integration test that verifies secrets are redacted from logs
- [P1] Path and file operation safeguards
  - Centralize secure path validation; forbid traversal outside allowed roots; validate symlinks
  - Expand tests for read/write/list to cover traversal attempts and symlink edge cases
- [P1] HTTP client configuration
  - reqwest: confirm rustls-tls everywhere, disable system proxies by default unless configured, set sane connect/read timeouts
  - Add retry policy with jitter (idempotent ops only); unit test retry policy boundaries
- [P1] Plugin system posture
  - Plugins currently disabled in engines; document rationale in README and code; ensure code paths cannot be enabled without explicit feature
  - If re-enabling in the future, gate behind audited feature flags and signature verification
- [P2] Rate limiting and backoff
  - Add per-engine optional rate limiter; expose minimal config; tests for burst and steady load


Testing and QA
- [P1] Stabilize and speed up CLI tests
  - tests/integration and tests/e2e_cli_tests.rs: prefer assert_cmd with cargo_bin to avoid repeated cargo run spawns where possible
  - Mark network/engine tests as ignored by default; provide a feature or env to enable
- [P1] Add unit tests for:
  - [DONE] crates/fluent-cli: command parsing for pipeline options (--input, --run-id, --force-fresh, --json)
  - [PARTIAL] crates/fluent-engines: engine selection/error mapping (unknown engine type test added); mock transport pending
  - [DONE] crates/fluent-core: config parsing, overrides, credentials/env merge
  - [PARTIAL] crates/fluent-agent: command allowlist/denylist tests added; MemorySystem and string_replace editor tests pending
- [P1] Add golden tests for response formatting and CSV/code extraction utilities
- [P2] Add property tests (proptests) for path validators and input sanitizers


Performance and Reliability
- [P1] Confirm connection pooling and client reuse across engines
  - Ensure a single reqwest::Client with tuned pool settings is shared where feasible
- [P1] Cache policy verification
  - crates/fluent-engines/src/enhanced_cache.rs and cache_manager.rs: document cache keying strategy; add tests for TTL, invalidation, and size limits
- [P2] Async ergonomics
  - Remove unused async fn warnings (e.g., clippy::unused_async) or justify why async is kept
  - Audit blocking operations on async paths (e.g., file IO in hot paths) and move to tokio fs if appropriate


CLI UX and Ergonomics
- [P1] Help and error messages
  - Ensure all subcommands have examples; unify tone and formatting; add --json where applicable
- [P1] Consistent exit codes
  - Map common error domains (config, network, engine unavailable) to consistent non-zero exit codes; test them
- [P2] Autocomplete scripts
  - Verify and document fluent_autocomplete.{sh,ps1}; add a CI step to regenerate if schema changes


Configuration and Docs
- [P1] Consolidate configuration documentation
  - Reconcile README engine names and actual config schema (names must match); add a “troubleshooting: engine not found” section
- [P1] Make examples resilient
  - Examples that require API keys: detect absence and print a single-line instruction with exit code 2 instead of failing deeper in stack
- [P2] Prune or clearly mark experimental Python frontends (frontend.py, frontend_secure.py)
  - Either move to examples/legacy or document their status and support policy
- [P2] Update docs on disabled plugin system and current MCP support level; add supported transports matrix


CI/CD and Tooling
- [P1] CI matrices
  - Validate current GitHub actions cover clippy and fmt; add cargo fmt --check and cargo clippy -D warnings jobs
  - Cache improvements: separate target dir per toolchain/target triple
- [P1] Release artifacts integrity
  - Ensure archives include only necessary runtime files; remove large unused assets; add checksums to release
- [P2] Pre-commit hooks
  - Add .pre-commit-config.yaml with rustfmt, clippy (via cargo clippy), and basic YAML/JSON linters; document usage


Code Hygiene
- [P1] Remove or feature-gate src/error_fixer.rs or move under examples/
- [P1] Reduce duplicate re-exports in root src/lib.rs if not needed by external users
- [P2] Audit dead code and cfg(test) markers; run cargo +stable clippy across workspace and fix new warnings
- [P2] Normalize workspace dependency versions
  - Ensure [workspace.dependencies] and member Cargo.toml agree on versions and feature sets; remove redundant per-crate pins where possible


Agentic, MCP, and Tools
- [P1] MCP integration hardening
  - Add health checks and structured logs around server startup; fail-fast on port conflicts or auth errors; tests for startup failures
- [P1] Tool system capability boundaries
  - Introduce per-tool capability config (max file size, path roots, command allowlist); add JSON schema for tool config
- [P2] String replace editor improvements
  - Add dry-run JSON diff output; add multi-pattern operations in single pass; test line range + case-insensitive combined


Neo4j and Storage
- [P1] Neo4j client robustness
  - crates/fluent-core/src/neo4j_client.rs: retry policy and idempotency; better error mapping; unit tests with mock server
- [P1] Centralize graph query validation helpers; add tests for is_valid_cypher and extract_cypher_query
- [P2] Storage module clarity
  - crates/fluent-storage: decide on sqlite support strategy; either add an async sqlite module or explicitly document why not supported


SDK and Lambda
- [P1] Fluent SDK request builder
  - crates/fluent-sdk/src/lib.rs: validate overrides and credentials schema; add explicit error types
- [P1] Lambda target
  - crates/fluent-lambda/src/main.rs: add cold start log, input size limits, and structured error body; document deployment steps and example payloads


Observability
- [P2] Logging
  - Standardize logging (tracing + env_logger or tracing-subscriber consistently); add span fields for request IDs where available
- [P2] Metrics
  - Optional metrics via prometheus feature flag; expose minimal counters (requests, errors, cache hits)


Repository Maintenance
- [P2] Remove large binary assets from repo or move to releases (e.g., output.png if not required)
- [P2] Ensure LICENSE references and third-party notices are up to date


Follow-ups After Changes
- Update README feature matrix and examples after major improvements (async memory store, CLI UX changes)
- Update TECHNICAL_DEBT.md and link to this TODO plan; keep both in sync until debt is cleared


Acceptance Criteria (Definition of Done)
- Clean cargo build on stable across workspace without deprecation warnings (except explicitly allowed ones)
- cargo test passes locally with networked tests gated behind a feature/env
- cargo clippy shows no new warnings; cargo fmt has no diffs
- CI runs lint, build, and tests across OS/targets; artifacts produced for release targets
- README and docs reflect current behavior precisely; examples succeed or exit gracefully with clear guidance
