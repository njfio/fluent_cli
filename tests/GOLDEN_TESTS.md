# Golden Tests Documentation

## Overview

Golden tests (also known as snapshot tests) are tests that verify the output format and structure of CLI commands remain consistent across changes. These tests help catch unintended changes to output formats that could break scripts or integrations that depend on them.

## Location

- Test file: `/tests/golden_tests.rs`
- Test configuration: `/tests/Cargo.toml`

## Running Golden Tests

```bash
# Run all golden tests
cargo test --test golden_tests

# Run specific golden test by name
cargo test --test golden_tests test_engine_list_json_format

# Run all JSON-related golden tests
cargo test --test golden_tests test_json

# List all available golden tests
cargo test --test golden_tests -- --list

# Run with output displayed
cargo test --test golden_tests -- --nocapture
```

## Test Categories

### 1. Help Output Format Tests

Tests that verify help text structure and content:
- `test_help_output_format` - Main CLI help output
- `test_agent_help_format` - Agent command help
- `test_tools_help_format` - Tools command help
- `test_engine_help_format` - Engine command help

**What they verify:**
- Help sections exist (Usage, Commands, Options)
- Expected commands are listed
- Help format is consistent

### 2. Engine List Format Tests

Tests for engine listing output:
- `test_engine_list_format` - Standard text output
- `test_engine_list_json_format` - JSON output structure

**What they verify:**
- JSON output is valid and well-structured
- Engine objects have required fields (name, engine, connection)
- Connection objects have required fields (hostname, port, protocol, etc.)

### 3. Tools List Format Tests

Tests for tool listing output:
- `test_tools_list_format` - Standard text output
- `test_tools_list_json_format` - JSON output structure
- `test_tools_list_with_filters_json_format` - Filtered output maintains format
- `test_tools_describe_json_format` - Tool description output

**What they verify:**
- JSON output structure (tools array, total_count field)
- Tool objects have required fields (name, description, executor)
- Filters maintain consistent output structure

### 4. Version Output Format Tests

Tests for version information:
- `test_version_output_format` - Version string format

**What they verify:**
- Version includes package name
- Version follows semantic versioning (X.Y.Z)

### 5. Schema Output Format Tests

Tests for JSON Schema generation:
- `test_schema_output_format` - Config schema output

**What they verify:**
- Schema is valid JSON
- Schema is a proper JSON Schema object

### 6. Completions Format Tests

Tests for shell completion scripts:
- `test_completions_bash_format` - Bash completions
- `test_completions_zsh_format` - Zsh completions

**What they verify:**
- Completions contain shell-specific syntax
- Output is properly formatted for each shell

### 7. Error Format Tests

Tests for error message consistency:
- `test_error_format_invalid_command` - Invalid command errors
- `test_error_format_missing_argument` - Missing argument errors

**What they verify:**
- Errors return non-zero exit codes
- Error messages contain helpful information

### 8. CSV Extraction Tests

Tests demonstrating JSON to CSV conversion:
- `test_json_to_csv_conversion_tools_list` - Tools list CSV extraction
- `test_json_to_csv_conversion_engine_list` - Engine list CSV extraction

**What they verify:**
- JSON structures have consistent fields across items
- Data can be reliably extracted to CSV format
- All items have the same schema (required for CSV)

## Test Design Philosophy

### Configuration Independence

Many tests are designed to work without requiring a full configuration file:
- `tools list` works with default tool registry
- `engine list` shows whatever engines are configured (or none)
- Help commands always work
- Version and completions commands are config-independent

### Graceful Degradation

Tests handle various scenarios gracefully:
- Missing configuration files
- Empty lists (no engines/tools)
- Commands that may not exist in all versions
- Optional features

### Structure Validation

Rather than exact string matching, tests validate:
- JSON structure and required fields
- Presence of expected sections in help
- Valid formatting patterns (e.g., version numbers)
- Consistency across items in lists

## Adding New Golden Tests

When adding new golden tests, follow these patterns:

### 1. Test Output Structure, Not Exact Content

```rust
// Good: Verify structure
assert!(json.get("field").is_some());
assert!(json["items"].is_array());

// Avoid: Exact string matching (too brittle)
// assert_eq!(stdout, "exact output");
```

### 2. Handle Optional Commands Gracefully

```rust
if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("not found") {
        return; // Skip test if command doesn't exist
    }
}
```

### 3. Validate JSON Schemas

```rust
let parsed: Result<Value, _> = serde_json::from_str(&stdout);
assert!(parsed.is_ok(), "Output should be valid JSON");

let json = parsed.unwrap();
assert!(json.is_object(), "Should be JSON object");
```

### 4. Test CSV Extractability

```rust
// Verify all items have consistent fields
for item in items {
    assert!(all_have_same_keys, "Required for CSV extraction");
}
```

## Maintenance

### When to Update Golden Tests

Update golden tests when:
- Intentionally changing output format
- Adding new required fields to JSON output
- Modifying help text structure
- Changing error message formats

### Breaking Changes

Changes that would break golden tests should be considered breaking changes to the CLI API:
- Removing fields from JSON output
- Changing JSON structure
- Removing help sections
- Changing exit codes

## Dependencies

Golden tests require:
- `assert_cmd` - CLI testing framework
- `serde_json` - JSON parsing
- `regex` - Pattern matching

## Test Coverage

Current coverage:
- **18 golden tests** covering:
  - 4 help format tests
  - 3 engine format tests
  - 4 tools format tests
  - 1 version format test
  - 1 schema format test
  - 2 completions format tests
  - 2 error format tests
  - 2 CSV extraction tests

## Future Enhancements

Potential additions:
- Snapshot testing with `insta` crate for exact output comparison
- Performance benchmarks for formatting operations
- More comprehensive CSV extraction tests
- Table format validation tests
- Markdown format validation tests
- Color/ANSI code stripping tests
