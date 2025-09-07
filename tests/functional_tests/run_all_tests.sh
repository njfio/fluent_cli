#!/bin/bash

# Test Runner for All Fluent CLI Functional Tests
# This script runs all the functional tests for the Fluent CLI

set -e  # Exit on any error

echo "🚀 Running All Fluent CLI Functional Tests"
echo "========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if fluent binary exists
if ! command -v fluent &> /dev/null; then
    echo -e "${YELLOW}⚠️  Fluent CLI binary not found in PATH${NC}"
    echo "Building fluent CLI..."
    cargo build --release
    # Add to PATH temporarily
    export PATH="$(pwd)/target/release:$PATH"
    
    if ! command -v fluent &> /dev/null; then
        echo -e "${RED}❌ Failed to build fluent CLI${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}✅ Fluent CLI binary found${NC}"

# Function to run a test suite
run_test_suite() {
    local name="$1"
    local command="$2"
    
    echo -e "\n${BLUE}▶️  Running $name${NC}"
    echo "----------------------------------------"
    
    if eval "$command"; then
        echo -e "${GREEN}✅ $name completed successfully${NC}"
        return 0
    else
        echo -e "${RED}❌ $name failed${NC}"
        return 1
    fi
}

# Track failures
FAILED_SUITES=()

# Run Rust-based functional tests
if run_test_suite "Rust Functional Tests" "cargo test --test cli_functional_tests"; then
    echo -e "${GREEN}✅ Rust functional tests passed${NC}"
else
    FAILED_SUITES+=("Rust Functional Tests")
fi

# Run shell script tests
if run_test_suite "Shell Script Tests" "bash ./test_all_cli_commands.sh"; then
    echo -e "${GREEN}✅ Shell script tests passed${NC}"
else
    FAILED_SUITES+=("Shell Script Tests")
fi

# Run Python scenario tests
if command -v python3 &> /dev/null; then
    if run_test_suite "Python Scenario Tests" "python3 ./test_cli_scenarios.py"; then
        echo -e "${GREEN}✅ Python scenario tests passed${NC}"
    else
        FAILED_SUITES+=("Python Scenario Tests")
    fi
else
    echo -e "${YELLOW}⚠️  Python 3 not found, skipping Python scenario tests${NC}"
fi

# Summary
echo -e "\n🏁 Test Execution Summary"
echo "========================"

if [ ${#FAILED_SUITES[@]} -eq 0 ]; then
    echo -e "${GREEN}🎉 All test suites passed!${NC}"
    exit 0
else
    echo -e "${RED}❌ The following test suites failed:${NC}"
    for suite in "${FAILED_SUITES[@]}"; do
        echo -e "${RED}  - $suite${NC}"
    done
    exit 1
fi