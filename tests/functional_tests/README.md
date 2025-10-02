# Fluent CLI Functional Tests

This directory contains comprehensive functional tests for the Fluent CLI that exercise all commands and options.

## Test Suites

### 1. Rust-Based Functional Tests (`cli_functional_tests.rs`)
- Uses `assert_cmd` to test CLI commands programmatically
- Tests all commands and their options
- Validates proper error handling
- Tests global options like `--help`, `--version`, `--config`

### 2. Comprehensive Option Tests (`comprehensive_option_tests.rs`)
- Detailed testing of every CLI option and combination
- Validates both long and short forms of all options
- Tests complex option interactions
- Ensures complete coverage of the CLI interface

### 3. Simple Test (`simple_test.rs`)
- Basic verification that the test infrastructure works
- Tests help and version commands

### 4. Shell Script Tests (`test_all_cli_commands.sh`)
- End-to-end testing using shell commands
- Tests all command combinations
- Provides colored output for easy identification of failures
- Tracks pass/fail statistics

### 5. Python Scenario Tests (`test_cli_scenarios.py`)
- Advanced scenario testing
- Complex command combinations
- Edge case testing
- JSON/YAML file generation for tests

### 6. Test Runner (`run_all_tests.sh`)
- Executes all test suites
- Provides summary of results
- Handles dependencies and setup

## Running Tests

### Run All Tests
```bash
cd /path/to/fluent_cli
./tests/functional_tests/run_all_tests.sh
```

### Run Individual Test Suites

#### Rust Tests
```bash
# Run all Rust-based tests
cargo test -p fluent-integration-tests --test cli_functional_tests
cargo test -p fluent-integration-tests --test comprehensive_option_tests
cargo test -p fluent-integration-tests --test simple_test

# Run specific test modules
cargo test -p fluent-integration-tests --test cli_functional_tests global_options_tests
cargo test -p fluent-integration-tests --test comprehensive_option_tests global_option_tests
```

#### Shell Script Tests
```bash


## Test Coverage

The functional tests cover:

1. **Global Options**
   - `--help` / `-h`
   - `--version` / `-V`
   - `--config` / `-c`

2. **Pipeline Command**
   - All subcommands and options
   - Required arguments validation
   - Optional arguments testing
   - Dry-run execution

3. **Agent Command**
   - Goal specification
   - Model selection
   - Iteration limits
   - Reflection mode
   - Dry-run execution

4. **MCP Command**
   - Server subcommand
   - Client subcommand
   - Port configuration

5. **Neo4j Command**
   - Cypher generation
   - Query execution
   - File upsert operations

6. **Tools Command**
   - List subcommand
   - Describe subcommand
   - Exec subcommand
   - Categories subcommand

7. **Engine Command**
   - List subcommand
   - Test subcommand

8. **Error Handling**
   - Invalid commands
   - Missing required arguments
   - Malformed options

9. **Complex Scenarios**
   - Multiple option combinations
   - Nested subcommands
   - Configuration file integration
   - Temporary file creation and cleanup

## Test Design Principles

1. **Non-Destructive Testing**
   - Uses temporary directories
   - Employs dry-run where possible
   - Does not require API keys or network access

2. **Comprehensive Coverage**
   - Tests all commands and options
   - Validates both success and failure cases
   - Tests edge cases and error conditions

3. **Self-Contained**
   - Creates all necessary test files
   - Cleans up after execution
   - Does not modify the source directory

4. **Clear Reporting**
   - Provides detailed pass/fail information
   - Uses colored output for better visibility
   - Tracks statistics across test runs

## Adding New Tests

To add new tests:

1. For simple command validation, add to `cli_functional_tests.rs`
2. For detailed option coverage, add to `comprehensive_option_tests.rs`
3. For end-to-end scenarios, add to `test_all_cli_commands.sh`
4. For complex scenarios, add to `test_cli_scenarios.py`
5. Update `run_all_tests.sh` if needed
6. Run all tests to ensure nothing is broken

## Dependencies

- Rust toolchain (for Rust tests)
- Python 3 (for Python tests)
- Bash-compatible shell (for shell tests)
- Fluent CLI binary in PATH

## Additional Documentation

For a comprehensive guide to all testing aspects, see [COMPREHENSIVE_TESTING_GUIDE.md](COMPREHENSIVE_TESTING_GUIDE.md)