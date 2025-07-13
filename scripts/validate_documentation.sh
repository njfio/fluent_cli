#!/bin/bash

# Documentation Validation Script
# Tests all documented CLI commands to ensure they work as documented

set -e

echo "🔍 Starting Documentation Validation..."
echo "========================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Test function
test_command() {
    local description="$1"
    local command="$2"
    local expected_exit_code="${3:-0}"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo -n "Testing: $description... "
    
    if eval "$command" >/dev/null 2>&1; then
        actual_exit_code=$?
    else
        actual_exit_code=$?
    fi
    
    if [ $actual_exit_code -eq $expected_exit_code ]; then
        echo -e "${GREEN}PASS${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
    else
        echo -e "${RED}FAIL${NC} (exit code: $actual_exit_code, expected: $expected_exit_code)"
        FAILED_TESTS=$((FAILED_TESTS + 1))
    fi
}

# Test help commands (should always work)
echo -e "\n${YELLOW}Testing Help Commands${NC}"
echo "------------------------"
test_command "Main help" "cargo run -- --help"
test_command "Agent help" "cargo run -- openai-gpt4 agent --help"
test_command "Tools help" "cargo run -- openai-gpt4 tools --help"
test_command "Pipeline help" "cargo run -- openai-gpt4 pipeline --help"
test_command "MCP help" "cargo run -- openai-gpt4 mcp --help"

# Test tool commands (should work without API keys)
echo -e "\n${YELLOW}Testing Tool Commands${NC}"
echo "------------------------"
test_command "Tools list" "cargo run -- openai-gpt4 tools list"
test_command "Tools categories" "cargo run -- openai-gpt4 tools categories"
test_command "Tool describe" "cargo run -- openai-gpt4 tools describe read_file"
test_command "Tool execution (file_exists)" "cargo run -- openai-gpt4 tools exec file_exists --path README.md"
test_command "Tool execution (cargo_check)" "cargo run -- openai-gpt4 tools exec cargo_check"

# Test JSON output
echo -e "\n${YELLOW}Testing JSON Output${NC}"
echo "------------------------"
test_command "Tools list JSON" "cargo run -- openai-gpt4 tools list --json"
test_command "Tool describe JSON" "cargo run -- openai-gpt4 tools describe read_file --json"

# Test examples
echo -e "\n${YELLOW}Testing Examples${NC}"
echo "------------------------"
test_command "Reflection demo" "cargo run --example reflection_demo"
test_command "State management demo" "cargo run --example state_management_demo"

# Test commands that require API keys (expect failure without keys)
echo -e "\n${YELLOW}Testing Commands Requiring API Keys (expect failures without keys)${NC}"
echo "------------------------------------------------------------------------"
test_command "Direct query (no API key)" "cargo run -- openai-gpt4 'test'" 1
test_command "Agent command (no API key)" "cargo run -- openai-gpt4 agent" 1

# Test with API key if available
if [ -n "$OPENAI_API_KEY" ]; then
    echo -e "\n${YELLOW}Testing Commands With API Key${NC}"
    echo "--------------------------------"
    test_command "Direct query (with API key)" "OPENAI_API_KEY='$OPENAI_API_KEY' cargo run -- openai-gpt4 'What is 2+2?'"
else
    echo -e "\n${YELLOW}Skipping API key tests (OPENAI_API_KEY not set)${NC}"
fi

# Test configuration validation
echo -e "\n${YELLOW}Testing Configuration${NC}"
echo "------------------------"
if [ -f "config.yaml" ]; then
    test_command "Config file exists" "test -f config.yaml"
    echo "✓ Config file found"
else
    echo -e "${RED}✗ Config file not found${NC}"
    FAILED_TESTS=$((FAILED_TESTS + 1))
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
fi

# Summary
echo -e "\n${YELLOW}Validation Summary${NC}"
echo "=================="
echo "Total tests: $TOTAL_TESTS"
echo -e "Passed: ${GREEN}$PASSED_TESTS${NC}"
echo -e "Failed: ${RED}$FAILED_TESTS${NC}"

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All documentation validation tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}❌ Some documentation validation tests failed.${NC}"
    echo "Please check the failed commands and update documentation or implementation."
    exit 1
fi
