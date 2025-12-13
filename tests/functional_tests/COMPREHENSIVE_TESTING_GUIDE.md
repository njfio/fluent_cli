# Comprehensive Functional Testing Guide for Fluent CLI

This document provides a detailed overview of the comprehensive functional testing suite for the Fluent CLI, ensuring all commands and options are thoroughly validated.

## Test Suite Overview

The Fluent CLI functional testing suite consists of multiple complementary test approaches:

1. **Rust-Based Functional Tests** (`cli_functional_tests.rs`) - Core command validation
2. **Comprehensive Option Tests** (`comprehensive_option_tests.rs`) - Detailed option coverage
3. **Shell Script Tests** (`test_all_cli_commands.sh`) - End-to-end command execution
4. **Python Scenario Tests** (`test_cli_scenarios.py`) - Advanced scenario testing
5. **Test Runner** (`run_all_tests.sh`) - Orchestrates all test suites

## Detailed Test Coverage

### Global Options

All global CLI options are tested for both long and short forms:

- `--help` / `-h` - Help documentation
- `--version` / `-V` - Version information
- `--config` / `-c` - Configuration file specification

### Pipeline Command

The pipeline command and all its options are comprehensively tested:

- `--file` / `-f` - Pipeline YAML file specification
- `--input` / `-i` - Input string for pipeline
- `--variables` / `-v` - Pipeline variables (multiple values supported)
- `--force-fresh` - Force fresh execution ignoring saved state
- `--run-id` - Optional run identifier
- `--dry-run` - Preview execution without running
- `--json` - JSON output format

### Agent Command

The agent command with all its extensive options:

- `--agentic` - Enable agentic mode
- `--preview` - Open generated artifact in viewer
- `--preview-path` - Path to preview file
- `--goal` / `-g` - Goal description
- `--goal-file` - Path to TOML goal file
- `--model` - Override model for engines
- `--max-iterations` - Maximum iteration count
- `--reflection` - Enable reflection mode
- `--enable-tools` - Enable tool usage
- `--agent-config` - Agent configuration JSON path
- `--dry-run` - Preview without side effects
- `--gen-retries` - Max LLM code generation retries
- `--min-html-size` - Minimum HTML size validation
- `--task` / `-t` - Specific task for agent

### MCP Command

Model Context Protocol command testing:

- `server` subcommand with `--port` / `-p` option
- `client` subcommand with `--server` / `-s` option

### Neo4j Command

Neo4j database operations:

- `--generate-cypher` - Generate Cypher from natural language
- `--query` / `-q` - Query specification
- `--upsert-file` - Input file for upsert operations

### Tools Command

Direct tool access with comprehensive subcommand testing:

#### List Subcommand
- `--category` - Filter by tool category
- `--search` - Search tools by name/description
- `--json` - JSON output format
- `--available` - Show only available tools
- `--detailed` - Show detailed information

#### Describe Subcommand
- `tool` - Required tool name argument
- `--json` - JSON output format
- `--schema` - Show tool schema/parameters
- `--examples` - Show usage examples

#### Exec Subcommand
- `tool` - Required tool name argument
- `--json-output` - Output result in JSON format

#### Categories Subcommand
- `--json` - JSON output format

### Engine Command

Engine management and configuration:

#### List Subcommand
- `--json` - JSON output format

#### Test Subcommand
- `engine` - Required engine name argument

## Test Design Principles

### 1. Comprehensive Coverage
- Every CLI option is tested in both long and short forms where applicable
- All subcommands are validated for correct behavior
- Edge cases and error conditions are tested
- Complex option combinations are validated

### 2. Non-Destructive Testing
- Temporary directories are used for all file operations
- Dry-run options are used where possible to avoid actual execution
- No API keys or network access required for basic functionality tests
- Cleanup is performed after each test

### 3. Self-Contained Tests
- All necessary test files are created within the test suite
- No external dependencies on specific file paths
- Test configurations are generated as needed
- Each test can run independently

### 4. Clear Reporting
- Pass/fail status is clearly indicated for each test
- Colored output for shell scripts improves visibility
- Statistics are tracked across test runs
- Detailed error messages help with debugging

## Running the Tests

### Prerequisites
- Rust toolchain installed
- Python 3 (for Python tests)
- Bash-compatible shell (for shell tests)
- Fluent CLI binary in PATH

### Running All Tests
```bash
cd /path/to/fluent_cli
./tests/functional_tests/run_all_tests.sh
```

### Running Individual Test Suites

#### Rust Tests
```bash
# Run all Rust-based tests
cargo test --test cli_functional_tests --test comprehensive_option_tests

# Run specific test modules
cargo test --test cli_functional_tests global_options_tests
cargo test --test comprehensive_option_tests pipeline_option_tests
```

#### Shell Script Tests
```bash
./tests/functional_tests/test_all_cli_commands.sh
```

#### Python Tests
```bash
./tests/functional_tests/test_cli_scenarios.py
```

## Test Categories

### Basic Functionality Tests
- Help and version information
- Command parsing and validation
- Required argument checking
- Simple option validation

### Advanced Option Tests
- Complex option combinations
- Nested subcommand validation
- Configuration file integration
- JSON output validation

### Error Handling Tests
- Invalid command handling
- Missing required arguments
- Malformed option values
- Configuration errors

### Integration Tests
- Multiple command workflows
- Cross-command option interactions
- Temporary file creation and cleanup
- Exit code validation

## Adding New Tests

### For Simple Command Validation
1. Add new test functions to `cli_functional_tests.rs`
2. Group related tests in appropriate modules
3. Use the `CliFunctionalTestRunner` for setup
4. Validate both success and failure cases

### For Detailed Option Coverage
1. Add new test functions to `comprehensive_option_tests.rs`
2. Create specific modules for each command
3. Test both long and short forms of options
4. Validate option combinations

### For End-to-End Scenarios
1. Add new test cases to `test_all_cli_commands.sh`
2. Use the existing test framework functions
3. Include both success and failure scenarios
4. Update test statistics tracking

### For Complex Scenarios
1. Add new test functions to `test_cli_scenarios.py`
2. Use the `CLITestRunner` class for setup
3. Test complex file operations and JSON/YAML handling
4. Validate advanced error conditions

## Test Maintenance

### Keeping Tests Up-to-Date
1. Review tests when adding new CLI commands
2. Update option tests when modifying command interfaces
3. Add new test modules for new feature areas
4. Remove deprecated tests for removed functionality

### Troubleshooting Test Failures
1. Check that the fluent binary is properly built and in PATH
2. Verify temporary directory permissions
3. Ensure no conflicting processes are running
4. Review test output for specific error messages

## Performance Considerations

### Test Execution Time
- Most tests use dry-run to avoid actual execution
- Parallel test execution is supported through cargo
- Shell script tests run sequentially for consistent output
- Python tests are designed to be lightweight

### Resource Usage
- Temporary directories are automatically cleaned up
- Memory usage is minimal for command-line testing
- No persistent state is modified by tests
- Network access is avoided where possible

## Continuous Integration

The test suite is designed to work seamlessly with CI/CD pipelines:

1. **Build Verification** - Tests validate that the CLI builds correctly
2. **Regression Testing** - Ensures new changes don't break existing functionality
3. **Cross-Platform Compatibility** - Tests work on Linux, macOS, and Windows
4. **Automated Reporting** - Clear pass/fail indicators for CI systems

## Future Enhancements

Planned improvements to the test suite:

1. **Performance Testing** - Measure command execution times
2. **Load Testing** - Validate behavior under high concurrency
3. **Security Testing** - Validate secure handling of sensitive data
4. **Compatibility Testing** - Test across different Rust versions
5. **Integration Testing** - Test with actual LLM APIs (with mocking)

## Contributing to Tests

To contribute to the test suite:

1. Fork the repository
2. Create a new branch for your test additions
3. Follow the existing test patterns and conventions
4. Ensure all tests pass before submitting a pull request
5. Add documentation for new test categories
6. Update this guide when adding significant new functionality

## Support and Issues

For issues with the test suite:

1. Check that all prerequisites are installed
2. Verify the fluent binary builds correctly
3. Review test output for specific error messages
4. File issues on the project repository with detailed reproduction steps
