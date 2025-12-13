#!/bin/bash

# Comprehensive Functional Test Script for Fluent CLI
# Tests all commands and options to ensure they work correctly

set -e  # Exit on any error
set -o pipefail  # Ensure pipeline failures propagate
echo "🧪 Fluent CLI Comprehensive Functional Tests"
echo "==========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test tracking
PASSED=0
FAILED=0
TOTAL=0

# Function to run a test
run_test() {
    local test_name="$1"
    local command="$2"
    local expected_exit_code="${3:-0}"

    TOTAL=$((TOTAL + 1))
    echo -e "${BLUE}Running test: $test_name${NC}"
    echo "Command: $command"

    # Run the command and capture exit code
    if eval "$command" >/dev/null 2>&1; then
        exit_code=0
    else
        exit_code=$?
    fi

    # Check if exit code matches expected
    if [ $exit_code -eq $expected_exit_code ]; then
        echo -e "${GREEN}✅ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌ FAILED (exit code: $exit_code, expected: $expected_exit_code)${NC}"
        FAILED=$((FAILED + 1))
    fi
    echo ""
}

# Function to run a test that should succeed
run_success_test() {
    run_test "$1" "$2" 0
}

# Function to run a test that should succeed in parsing
run_parse_test() {
    local test_name="$1"
    local command="$2"

    TOTAL=$((TOTAL + 1))
    echo -e "${BLUE}Running test: $test_name${NC}"
    echo "Command: $command"

    # Run the command and capture exit code
    if eval "$command" >/dev/null 2>&1; then
        exit_code=0
    else
        exit_code=$?
    fi

    # For parsing tests, we're mainly checking that the command is recognized
    # Exit code 2 typically means argument parsing issues, which we want to catch
    # Exit codes 0 or other values might be OK for parsing tests
    if [ $exit_code -ne 2 ]; then
        echo -e "${GREEN}✅ PASSED${NC}"
        PASSED=$((PASSED + 1))
    else
        echo -e "${RED}❌ FAILED (exit code: $exit_code)${NC}"
        FAILED=$((FAILED + 1))
    fi
    echo ""
}

# Function to run a test that should fail
run_failure_test() {
    local expected_code="${3:-1}"
    run_test "$1" "$2" $expected_code
}

# Create temporary directory for test files
TEST_DIR=$(mktemp -d)
echo "Using temporary directory: $TEST_DIR"
cd "$TEST_DIR"

# Create test configuration files
cat > test_config.yaml << 'EOF'
engines:
- name: test-engine
  engine: openai
  connection:
    protocol: https
    hostname: api.openai.com
    port: 443
    request_path: /v1/chat/completions
  parameters:
    bearer_token: "test-token"
    modelName: gpt-3.5-turbo
    max_tokens: 1000
    temperature: 0.7
EOF

cat > test_pipeline.yaml << 'EOF'
name: test_pipeline
steps:
  - !Command
    name: test_step
    command: echo "Hello, world!"
EOF

cat > test_goal.toml << 'EOF'
goal_description = "Create a simple function"
max_iterations = 5
success_criteria = ["Function compiles without errors"]
EOF

# Test 1: Global Options
echo "📋 Testing Global Options"
echo "========================"

run_success_test "Help option (--help)" "fluent --help"
run_success_test "Help option (-h)" "fluent -h"
run_success_test "Version option (--version)" "fluent --version"
run_success_test "Version option (-V)" "fluent -V"
run_success_test "Config option (--config)" "fluent --config test_config.yaml --help"
run_success_test "Config option (-c)" "fluent -c test_config.yaml --help"

# Test 2: Pipeline Command
echo "📋 Testing Pipeline Command"
echo "=========================="

run_success_test "Pipeline help" "fluent pipeline --help"
run_failure_test "Pipeline without required --file" "fluent pipeline" 2
run_parse_test "Pipeline with all options (dry-run)" "fluent pipeline --file test_pipeline.yaml --config test_config.yaml --input 'test input' --variables key1=value1 --variables key2=value2 --force-fresh --run-id test-run-123 --dry-run --json"

# Test 3: Agent Command
echo "📋 Testing Agent Command"
echo "========================"

run_success_test "Agent help" "fluent agent --help"
run_parse_test "Agent with goal and options (dry-run)" "fluent agent --goal 'Create a simple function' --max-iterations 5 --reflection --dry-run --config test_config.yaml"
run_parse_test "Agent with goal file and options (dry-run)" "fluent agent --goal-file test_goal.toml --model gpt-4o --gen-retries 2 --min-html-size 1000 --dry-run --config test_config.yaml"

# Test 4: MCP Command
echo "📋 Testing MCP Command"
echo "======================"

run_success_test "MCP help" "fluent mcp --help"
run_success_test "MCP server help" "fluent mcp server --help"
run_success_test "MCP client help" "fluent mcp client --help"

# Test 5: Neo4j Command
echo "📋 Testing Neo4j Command"
echo "========================"

run_success_test "Neo4j help" "fluent neo4j --help"
run_parse_test "Neo4j generate-cypher option" "fluent neo4j --generate-cypher --query 'Find all users' --config test_config.yaml"
run_parse_test "Neo4j upsert-file option" "fluent neo4j --upsert-file test.txt --config test_config.yaml"

# Test 6: Tools Command
echo "📋 Testing Tools Command"
echo "========================"

run_success_test "Tools help" "fluent tools --help"
run_success_test "Tools list help" "fluent tools list --help"
run_success_test "Tools describe help" "fluent tools describe --help"
run_success_test "Tools exec help" "fluent tools exec --help"
run_success_test "Tools categories help" "fluent tools categories --help"

# Test 7: Engine Command
echo "📋 Testing Engine Command"
echo "========================="

run_success_test "Engine help" "fluent engine --help"
run_success_test "Engine list help" "fluent engine list --help"
run_success_test "Engine test help" "fluent engine test --help"
run_success_test "Engine list with json" "fluent engine list --json"

# Test 8: Error Handling
echo "📋 Testing Error Handling"
echo "========================="

run_failure_test "Invalid top-level command" "fluent invalid-command" 2
run_failure_test "Invalid subcommand" "fluent pipeline invalid-subcommand" 2
run_failure_test "Missing required args" "fluent pipeline" 2

# Test 9: Complex Command Combinations
echo "📋 Testing Complex Command Combinations"
echo "======================================"

run_success_test "Multiple global options" "fluent --config test_config.yaml --help"
run_parse_test "Nested subcommands" "fluent --config test_config.yaml tools list --json"
run_success_test "All major commands help" "fluent pipeline --help && fluent agent --help && fluent mcp --help && fluent neo4j --help && fluent tools --help && fluent engine --help"

# Test 10: Comprehensive Tools Command Testing
echo "📋 Testing Comprehensive Tools Command"
echo "====================================="

run_parse_test "Tools list with all options" "fluent --config test_config.yaml tools list --category file --search read --json --available --detailed"
run_parse_test "Tools describe with all options" "fluent --config test_config.yaml tools describe read_file --json --schema --examples"
run_parse_test "Tools exec with options" "fluent --config test_config.yaml tools exec read_file --json-output"
run_parse_test "Tools categories with json" "fluent --config test_config.yaml tools categories --json"

# Test 11: Comprehensive Engine Command Testing
echo "📋 Testing Comprehensive Engine Command"
echo "======================================"

run_parse_test "Engine list with json" "fluent --config test_config.yaml engine list --json"
# Note: Engine test will fail without valid config, but we test it parses correctly
run_parse_test "Engine test nonexistent" "fluent --config test_config.yaml engine test nonexistent-engine"

# Test 12: Advanced Option Combinations
echo "📋 Testing Advanced Option Combinations"
echo "======================================"

run_parse_test "Complex nested commands" "fluent --config test_config.yaml tools list --category file --json | grep read"
run_success_test "Multiple option combinations" "fluent --help | grep -i fluent"

# Cleanup
cd /
rm -rf "$TEST_DIR"

# Summary
echo "🏁 Test Summary"
echo "==============="
echo "Total tests: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}🎉 All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ Some tests failed.${NC}"
    exit 1
fi
