# Working with Rust Diagnostics

This guide explains the recommended approaches for working with Rust compiler diagnostics and automated error fixing in the fluent_cli project.

## Background

The project previously included a custom error fixer (`examples/legacy/error_fixer.rs`) that attempted to parse and fix Rust compiler errors through string matching and manual file manipulation. This approach has been deprecated in favor of Rust's built-in tooling, which provides superior reliability, coverage, and maintainability.

## Recommended Approaches

### 1. Automatic Fixes with `cargo fix`

The `cargo fix` command automatically applies compiler-suggested fixes to your code.

```bash
# Apply automatic fixes to your code
cargo fix

# Allow fixes even with uncommitted changes
cargo fix --allow-dirty

# Fix tests as well
cargo fix --all-targets

# Preview what would be fixed without applying changes
cargo fix --dry-run
```

**What it fixes:**
- Unused imports and variables
- Deprecated API usage
- Edition migrations
- Clippy suggestions (when used with `cargo clippy --fix`)

**Example workflow:**
```bash
# Check for issues
cargo check

# Automatically fix what's possible
cargo fix --allow-dirty

# Fix clippy warnings too
cargo clippy --fix --allow-dirty

# Verify everything builds
cargo build
```

### 2. Structured Diagnostics with JSON Output

For programmatic error handling or custom tooling, use structured JSON diagnostics:

```bash
# Get JSON-formatted diagnostics
cargo check --message-format=json

# Pretty-print with jq
cargo check --message-format=json 2>&1 | jq

# Filter to only errors
cargo check --message-format=json 2>&1 | jq 'select(.reason == "compiler-message" and .message.level == "error")'

# Extract specific fields
cargo check --message-format=json 2>&1 | jq '.message | select(.level == "error") | {file: .spans[0].file_name, line: .spans[0].line_start, message: .message}'
```

**JSON Diagnostic Structure:**
```json
{
  "reason": "compiler-message",
  "package_id": "fluent-cli 0.1.0",
  "target": {
    "kind": ["bin"],
    "name": "fluent"
  },
  "message": {
    "rendered": "...",
    "children": [...],
    "code": {
      "code": "E0308",
      "explanation": "..."
    },
    "level": "error",
    "message": "mismatched types",
    "spans": [
      {
        "file_name": "src/main.rs",
        "line_start": 42,
        "line_end": 42,
        "column_start": 5,
        "column_end": 23,
        "is_primary": true,
        "text": [{"text": "    let x: bool = 42;", "highlight_start": 5, "highlight_end": 23}],
        "suggested_replacement": null
      }
    ]
  }
}
```

### 3. IDE Integration with rust-analyzer

[rust-analyzer](https://rust-analyzer.github.io/) provides real-time diagnostics and quick fixes in your editor.

**Features:**
- Real-time error checking as you type
- Inline error messages and suggestions
- Quick fixes (code actions) for common issues
- Code completion and navigation
- Refactoring support

**Setup:**
- **VSCode**: Install the "rust-analyzer" extension
- **IntelliJ/CLion**: Built-in support
- **Vim/Neovim**: Use coc-rust-analyzer or native LSP
- **Emacs**: Use lsp-mode or eglot

### 4. Clippy for Advanced Linting

Clippy provides additional lints beyond the compiler's built-in checks:

```bash
# Run clippy
cargo clippy

# Treat warnings as errors
cargo clippy -- -D warnings

# Fix clippy suggestions automatically
cargo clippy --fix --allow-dirty

# Run clippy on all targets (including tests)
cargo clippy --all-targets -- -D warnings
```

**Common Clippy Lints:**
- Unused code and imports
- Inefficient algorithms
- Idiomatic improvements
- Potential bugs and correctness issues
- Style and readability

### 5. Testing and Validation

Always verify fixes with comprehensive testing:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test suite
cargo test --test integration

# Run tests for specific crate
cargo test -p fluent-agent

# Check formatting
cargo fmt --all -- --check

# Apply formatting
cargo fmt --all
```

## Integration with fluent_cli Tools

The fluent_cli project includes Rust tooling integration through the agent system:

### Available Rust Tools

From `fluent-agent/src/tools/rust.rs`:

1. **cargo_check** - Run `cargo check` to validate code
2. **cargo_build** - Build the project
3. **cargo_test** - Run tests
4. **cargo_run** - Execute the binary
5. **cargo_fix** - Apply automatic fixes
6. **cargo_clippy** - Run linter

### Example: Using Tools in Agent Mode

```bash
# Start agent mode
cargo run -- agent

# In agent mode, tools can be invoked:
# - cargo_check: Validates the codebase
# - cargo_fix: Applies automatic fixes
# - cargo_test: Runs test suite
```

### Programmatic Usage

```rust
use fluent_agent::tools::rust::CargoCheckTool;
use fluent_agent::tools::Tool;

// Create cargo check tool
let check_tool = CargoCheckTool;

// Execute and get diagnostics
let result = check_tool.execute(&params).await?;
```

## Best Practices

### 1. Iterative Fix Workflow

```bash
# 1. Check for errors
cargo check

# 2. Apply automatic fixes
cargo fix --allow-dirty

# 3. Run clippy for additional issues
cargo clippy --fix --allow-dirty

# 4. Format code
cargo fmt --all

# 5. Run tests
cargo test

# 6. Final validation
cargo clippy --all-targets -- -D warnings
```

### 2. Pre-Commit Hooks

Consider using pre-commit hooks to catch issues early:

```bash
# Install pre-commit hook (if using git hooks)
# Add to .git/hooks/pre-commit:
#!/bin/bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

### 3. CI/CD Integration

The project includes CI validation (see `.github/workflows/` if present):

```yaml
- name: Check formatting
  run: cargo fmt --all -- --check

- name: Run clippy
  run: cargo clippy --all-targets -- -D warnings

- name: Run tests
  run: cargo test
```

### 4. Handling Complex Errors

For errors that can't be auto-fixed:

1. **Read the error message carefully** - Rust errors are detailed and helpful
2. **Check the error code** (e.g., E0308) - Look it up with `rustc --explain E0308`
3. **Review compiler suggestions** - Often includes exact fix instructions
4. **Use rust-analyzer quick fixes** - Many complex issues have one-click solutions
5. **Consult documentation** - The Rust Book, API docs, or community resources

## Why Not Custom Error Fixers?

The deprecated `error_fixer.rs` demonstrates why custom error fixing is problematic:

### Problems with Custom Fixers:
- **Brittle**: Hardcoded line numbers break with any code change
- **Limited**: Only handles specific, known error patterns
- **Unreliable**: String parsing misses edge cases
- **Dangerous**: Direct file manipulation can corrupt code
- **Redundant**: Duplicates existing, superior tooling

### Advantages of Built-in Tools:
- **Comprehensive**: Handles hundreds of error types
- **Maintained**: Updated with each Rust release
- **Reliable**: Tested extensively by the Rust community
- **Safe**: Uses compiler's understanding of code structure
- **Integrated**: Works seamlessly with IDE and CI/CD

## Additional Resources

- [Rust Compiler Error Index](https://doc.rust-lang.org/error-index.html)
- [cargo fix Documentation](https://doc.rust-lang.org/cargo/commands/cargo-fix.html)
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/)
- [rust-analyzer User Manual](https://rust-analyzer.github.io/manual.html)
- [The Rust Book - Appendix D: Useful Development Tools](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html)

## Summary

For Rust error fixing in fluent_cli:

1. **Primary tool**: `cargo fix --allow-dirty`
2. **Linting**: `cargo clippy --fix --allow-dirty`
3. **IDE**: Use rust-analyzer for real-time feedback
4. **Programmatic**: Use `--message-format=json` for custom tooling
5. **Never**: Write custom string-based error fixers

The Rust ecosystem provides mature, reliable, and comprehensive tooling for error diagnostics and fixing. Use these tools instead of reinventing the wheel.
