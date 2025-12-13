# Semantic Validation System for Generated Code

This document describes the semantic validation system implemented in `crates/fluent-cli/src/code_validation.rs`.

## Overview

The validation system provides comprehensive semantic validation for generated code across multiple programming languages. It checks syntax markers, requirements, and code quality to ensure generated code meets minimum standards.

## Key Components

### 1. ValidationResult Struct

```rust
pub struct ValidationResult {
    pub valid: bool,           // Whether code passes all checks
    pub score: f32,           // Quality score from 0.0 to 1.0
    pub issues: Vec<String>,   // List of validation issues
    pub suggestions: Vec<String>, // Improvement suggestions
}
```

**Score Calculation:**
- Score = (checks_passed / total_checks)
- Validity threshold: 70% (score >= 0.7)

### 2. Main Validation Function

```rust
pub fn validate_generated_code(
    code: &str,
    language: &str,
    requirements: &[&str],
) -> ValidationResult
```

**Parameters:**
- `code`: The generated code to validate
- `language`: Programming language (rust, python, javascript, lua, html)
- `requirements`: Array of keywords/features that must be present

**Returns:** ValidationResult with detailed feedback

## Supported Languages

### 1. Rust Validation

**Checks:**
- Function definitions (`fn main()` or `fn `)
- Balanced braces `{}`
- Variable declarations (`let `, `mut `)

**Minimum Size:** 100 characters

### 2. Python Validation

**Checks:**
- Function or class definitions (`def `, `class `)
- Proper indentation (4 or 8 spaces, or tabs)
- Import statements (`import `, `from `)

**Minimum Size:** 50 characters

### 3. JavaScript Validation

**Checks:**
- Function or variable declarations (`function `, `const `, `let `, `var `)
- Balanced braces `{}`
- JavaScript syntax markers (`;`, `=>`)

**Minimum Size:** 50 characters

### 4. Lua Validation

**Checks:**
- Function or local declarations (`function `, `local `)
- Love2D callbacks (`love.load`, `love.draw`, `love.update`)
- Proper end statements (matching function count)

**Minimum Size:** 50 characters

### 5. HTML Validation

**Checks:**
- HTML document structure (`<html`, `<!doctype html`)
- Head and body tags (`<head`, `<body`)
- Embedded scripts or styles (`<script`, `<style`)
- Canvas element (`<canvas`) for games

**Minimum Size:** 500 characters

## Requirements Validation

The system checks that code contains specified keywords/patterns:

```rust
let result = validate_generated_code(
    code,
    "rust",
    &["main", "println", "struct"]  // These must appear in code
);
```

Each requirement is checked case-insensitively. Missing requirements are reported as issues with suggestions.

## Usage Example

```rust
use fluent_cli::code_validation::validate_generated_code;

let rust_code = r#"
    fn main() {
        let message = "Hello, world!";
        println!("{}", message);
    }
"#;

let result = validate_generated_code(
    rust_code,
    "rust",
    &["main", "println"]
);

if result.valid {
    println!("Code is valid! Score: {:.1}%", result.score * 100.0);
} else {
    println!("Validation failed:");
    for issue in &result.issues {
        println!("  - {}", issue);
    }
    println!("Suggestions:");
    for suggestion in &result.suggestions {
        println!("  - {}", suggestion);
    }
}
```

## Integration with Agentic Mode

The validation system can be integrated into the game creation flow in `agentic.rs`:

```rust
// After generating game code
let requirements = match expected_game {
    "tetris" => vec!["tetromino", "grid", "rotate"],
    "snake" => vec!["snake", "food", "direction"],
    "pong" => vec!["paddle", "ball"],
    _ => vec!["update", "draw", "input"],
};

let validation_result = validate_generated_code(
    &game_code,
    file_extension,
    &requirements,
);

if !validation_result.valid {
    // Request code refinement with specific issues
    for issue in &validation_result.issues {
        log(format!("Issue: {}", issue));
    }
}
```

## Test Coverage

The module includes comprehensive tests for:

1. Valid code in each supported language
2. Invalid code (too short)
3. Missing requirements
4. Edge cases (unbalanced braces, missing syntax)

Run tests with:
```bash
cargo test -p fluent-cli code_validation
```

## Future Enhancements

Potential improvements:

1. **Advanced Syntax Parsing:** Use tree-sitter for proper AST-based validation
2. **Security Checks:** Detect dangerous patterns (SQL injection, command injection)
3. **Performance Checks:** Detect O(n²) loops, memory leaks
4. **Style Checks:** Enforce naming conventions, documentation
5. **Custom Rules:** Allow users to define validation rules via config files
6. **Language-Specific Linters:** Integration with rustfmt, black, eslint, etc.

## API Reference

### ValidationResult Methods

- `new(valid: bool, score: f32) -> Self` - Create new validation result
- `add_issue(&mut self, issue: String)` - Add validation issue
- `add_suggestion(&mut self, suggestion: String)` - Add improvement suggestion
- `calculate_score(checks_passed: usize, total_checks: usize) -> f32` - Calculate score

### Public Functions

- `validate_generated_code(code: &str, language: &str, requirements: &[&str]) -> ValidationResult`
  - Main validation entry point

### Internal Functions

- `validate_rust_syntax(code_lower: &str) -> Vec<SyntaxCheck>`
- `validate_python_syntax(code_lower: &str) -> Vec<SyntaxCheck>`
- `validate_javascript_syntax(code_lower: &str) -> Vec<SyntaxCheck>`
- `validate_lua_syntax(code_lower: &str) -> Vec<SyntaxCheck>`
- `validate_html_syntax(code_lower: &str) -> Vec<SyntaxCheck>`
- `validate_requirements(code_lower: &str, requirements: &[&str]) -> Vec<SyntaxCheck>`

## Module Location

- **Implementation:** `crates/fluent-cli/src/code_validation.rs`
- **Module Export:** `crates/fluent-cli/src/lib.rs`
- **Public API:** Exported as `fluent_cli::code_validation::validate_generated_code`
- **Re-exports:** Available as `fluent_cli::{validate_generated_code, ValidationResult}`

## Design Principles

1. **Extensible:** Easy to add new languages
2. **Detailed Feedback:** Provides specific issues and suggestions
3. **Configurable:** Minimum sizes and thresholds can be adjusted
4. **Fast:** Lightweight string-based checks (no heavy parsing)
5. **Practical:** Focuses on common issues in generated code
