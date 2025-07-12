---
type: "always_apply"
---

Rust Coding Standards for Fluent CLI
General Principles
Safety and Reliability: Prioritize safe code. Avoid unsafe blocks unless absolutely necessary and justify their use with comments. Use Rust's ownership and borrowing system to prevent common errors.
Error Handling: Never use .unwrap() or .expect() in production code. Always handle errors properly using Result and ? operator. Propagate errors up the call stack where appropriate, and provide meaningful error messages.
Performance: Optimize for efficiency, especially in CLI tools where speed matters. Use efficient data structures and avoid unnecessary allocations.
Modularity: Keep code modular. Each crate should have a single responsibility (e.g., fluent-core for utilities, fluent-engines for LLM integrations).
Documentation: Every public function, struct, and module must have doc comments. Use /// for documentation and include examples where possible.
Formatting and Style
Rustfmt: All code must be formatted using rustfmt. Run cargo fmt before committing.
Clippy: Enforce Clippy lints. Run cargo clippy --all -- -D warnings and fix all warnings.
Naming Conventions:
Use snake_case for variables, functions, and modules.
Use CamelCase for types (structs, enums, traits).
Use SCREAMING_SNAKE_CASE for constants.
Avoid abbreviations unless they are standard (e.g., ctx for context).
Line Length: Limit lines to 100 characters where possible.
Imports: Group imports: standard library first, then third-party, then local. Use fully qualified paths for clarity in ambiguous cases.
Code Structure
Modules: Use modules to organize code logically. Each file should correspond to a module or a small set of related items.
Traits and Impl: Define traits for interfaces and implement them for types. Favor composition over inheritance.
Generics and Lifetimes: Use generics for type safety and flexibility. Annotate lifetimes explicitly when needed.
Concurrency: Use std::sync or Tokio for async where appropriate. Ensure thread-safety in shared state.
Specific to Fluent CLI
CLI Parsing: Use Clap for argument parsing. Structure commands modularly (e.g., subcommands for different engines).
LLM Integrations: Abstract LLM providers behind a trait (e.g., Engine trait in fluent-engines). Handle API keys securely, never hardcode them.
Security: Validate all inputs (e.g., using validators for URLs, paths). Use secure random for sessions. Pin dependencies to avoid supply-chain attacks.
Testing: Aim for 80%+ code coverage. Write unit tests for pure functions, integration tests for CLI commands.
Dependencies: Minimize external dependencies. Review and audit third-party crates regularly.
Logging: Use the log crate with levels (debug, info, error). Make logging configurable.
Bad Habits to Avoid
No global mutable state unless protected by mutex.
Avoid panics; use errors instead.
Don't ignore return values of functions that return Results.
Avoid deep nesting; use early returns.
No duplicated code; refactor into functions or macros.