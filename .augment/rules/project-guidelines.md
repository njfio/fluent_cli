---
type: "always_apply"
---

Project Guidelines for Fluent CLI
Project Overview
Fluent CLI is a Rust-based CLI tool for interacting with workflow systems and LLM providers. It emphasizes security, modularity, and ease of use.

Contribution Process
Fork and PR: Fork the repo, create a branch for your feature/bugfix, and submit a PR.
Commit Messages: Use conventional commits (e.g., "feat: add new engine", "fix: handle error in parsing").
Branch Naming: Use descriptive names like feature/add-zapier-support or bugfix/fix-session-id.
Code Reviews: All PRs require at least one approval. Address all comments before merging.
CI/CD: Ensure all tests pass. Use GitHub Actions for builds and tests.
Project Structure
Crates: Use a workspace with multiple crates:
fluent-cli: Main CLI entrypoint.
fluent-core: Shared utilities and configs.
fluent-engines: LLM engine implementations.
fluent-agent: Agentic features.
fluent-storage: Persistence layer (e.g., SQLite).
fluent-sdk: For external use.
Docs: Maintain docs in docs/ with subfolders for analysis, guides, implementation, security, testing.
Scripts: QA scripts in scripts/ (e.g., security audits).
Tests: In tests/ for integration; unit tests in each crate.
Examples: Demos in examples/.
Development Practices
Versioning: Follow SemVer. Update Cargo.toml accordingly.
Dependencies: Pin versions in Cargo.lock. Review updates for security.
Security: Follow best practices: input sanitization, secure storage of secrets, regular audits.
Performance: Profile with tools like cargo flamegraph. Optimize hotspots.
Accessibility: Ensure CLI output is readable (e.g., color optional, support for screen readers).
Internationalization: Prepare for i18n if expanding.
Tools and Environment
Rust Version: Use the latest stable Rust (e.g., 1.79+).
Build: Use cargo build and cargo test.
Linting: cargo fmt, cargo clippy.
Documentation: Use cargo doc for API docs.
Goals for Success
Maintain high test coverage and zero warnings.
Keep the codebase clean and readable.
Encourage community contributions with clear guidelines.
Regularly release updates with changelogs.