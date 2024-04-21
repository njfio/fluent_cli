#!/bin/bash

# Configuration
CLI_PATH="/Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent"
TEST_DATA_PATH="/Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests"
CSV_FILE="/Users/n/Downloads/functional_test_results.csv"
LOG_FILE="/Users/n/Downloads/functional_test_log.txt"
SYSTEM_PROMPT_FILE="$TEST_DATA_PATH/functional_test_spanish_system_prompt.txt"
OUTLINE_FILE="$TEST_DATA_PATH/functional_test_outline.txt"
CONTEXT_FILE="$TEST_DATA_PATH/functional_test_context.txt"

# Validation CLI and Flowname
VALIDATION_CLI="fluent"
VALIDATION_FLOWNAME="HaikuToolAgentRepoCloud"

# Flow names array
declare -a FLOWNAMES=("GroqMixtral8x7bAgentAnotherWebService" "SonnetXMLAgentAnowtherWebService" "GroqLLama370b8192AgentAnotherWebService" "MistralLargeToolAgentAnowtherWebService" "GPT4FunctionAgentWithMemoryAndBrowsing")

# Start new log file
echo "Starting new test session at $(date)" > "$LOG_FILE"

# Initialize CSV
echo "FlowName,TestID,Result,Runtime(s)" > "$CSV_FILE"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Initialize a counter for the test number
test_number=0

# Helper function for running a single test
run_test() {

    local flowname="$1"
    local test_id="$2"
    local command="$3"
    local validation_command="$4"

    # Increment the test counter
    ((test_number++))

    local test_start_time=$(date +%s)  # Start time for this test

    echo -e "${BOLD}=================================================================================================${NC}" | tee -a "$LOG_FILE"
    echo -e "${GREEN}Test Number $test_number: Testing $test_id for Flow: $flowname${NC}" | tee -a "$LOG_FILE"
    echo -e "${BOLD}-------------------------------------------------------------------------------------------------${NC}" | tee -a "$LOG_FILE"

    local test_output=$(eval "$command" | tee -a "$LOG_FILE" | tee /dev/tty | eval "$validation_command")
    local result=$(echo "$test_output" | grep -oE "PASS|FAIL")

    local test_end_time=$(date +%s)  # End time for this test
    local test_runtime=$((test_end_time - test_start_time))  # Runtime for this test

    if [[ "$result" == "PASS" ]]; then
        echo -e "${GREEN}Test Number $test_number: $result    Tested $test_id${NC}" | tee -a "$LOG_FILE"
    else
        echo -e "${RED}Test Number $test_number: $result    Tested $test_id${NC}" | tee -a "$LOG_FILE"
    fi

    echo -e "${BOLD}=================================================================================================${NC}" | tee -a "$LOG_FILE"
    echo "$flowname,$test_id,$result,$test_runtime" >> "$CSV_FILE"  # Log result with runtime
    sleep 1
}

# Loop through each flow name
for FLOWNAME in "${FLOWNAMES[@]}"; do
    echo ""
    echo ""
    echo ""
    echo -e "\t\t\t\t\t****Running tests for $FLOWNAME****" | tee -a "$LOG_FILE"
    echo ""
    echo ""
    echo ""

    run_test "$FLOWNAME" "Base Command Test" \
        "$CLI_PATH $FLOWNAME 'This is a test, respond that this is a test'" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request is about this is a test'"

    run_test "$FLOWNAME" "Stdin Context Test" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'This is the content: '" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request has the word northstar or North Star'"

    run_test "$FLOWNAME" "Additional Context File Test" \
        "$CLI_PATH $FLOWNAME 'What is the content: ' --additional-context-file \"$OUTLINE_FILE\"" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains the word TheLardCatFellFlatOnTheMat'"

    run_test "$FLOWNAME" "Combined Stdin and Additional Context Test" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'What are these contents about:' --additional-context-file \"$OUTLINE_FILE\"" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains TheLardCatFellFlatOnTheMat and talks about the word northstar or North Star'"

    run_test "$FLOWNAME" "Base Command Test and --system-prompt-override-inline" \
        "$CLI_PATH $FLOWNAME 'This is a test, respond that this is a test' --system-prompt-override-inline 'You can only reply in german'" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request is about this is a test and is in german'"

    run_test "$FLOWNAME" "Stdin Context Test and --system-prompt-override-inline" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'This is the content: ' --system-prompt-override-inline 'You can only reply in german' " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request has the word northstar or North Star and is in german'"

    run_test "$FLOWNAME" "Additional Context File Test and --system-prompt-override-inline" \
        "$CLI_PATH $FLOWNAME 'What is the content:' --additional-context-file \"$OUTLINE_FILE\" --system-prompt-override-inline 'You can only reply in german' " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains the word TheLardCatFellFlatOnTheMat and is in german'"

    run_test "$FLOWNAME" "Combined Stdin and Additional Context Test and --system-prompt-override-inline" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'What are these contents about:' --additional-context-file \"$OUTLINE_FILE\" --system-prompt-override-inline 'You can only reply in german'" \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains TheLardCatFellFlatOnTheMat and talks about the word northstar or North Star and is in german'"

    run_test "$FLOWNAME" "Base Command Test and --system-prompt-override-file" \
        "$CLI_PATH $FLOWNAME 'This is a test, respond that this is a test' --system-prompt-override-file \"$SYSTEM_PROMPT_FILE\" " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request is about this is a test and is in spanish'"

    run_test "$FLOWNAME" "Stdin Context Test and --system-prompt-override-file" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'This is the content: ' --system-prompt-override-file \"$SYSTEM_PROMPT_FILE\" " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request has the word northstar or North Star and is in spanish'"

    run_test "$FLOWNAME" "Additional Context File Test and --system-prompt-override-file" \
        "$CLI_PATH $FLOWNAME 'What is the content: ' --additional-context-file \"$OUTLINE_FILE\" --system-prompt-override-file \"$SYSTEM_PROMPT_FILE\" " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains the word TheLardCatFellFlatOnTheMat and is in spanish'"

    run_test "$FLOWNAME" "Combined Stdin and Additional Context Test and --system-prompt-override-file" \
        "cat \"$CONTEXT_FILE\" | $CLI_PATH $FLOWNAME 'What are these contents about:' --additional-context-file \"$OUTLINE_FILE\" --system-prompt-override-file \"$SYSTEM_PROMPT_FILE\" " \
        "$VALIDATION_CLI $VALIDATION_FLOWNAME 'Answer PASS or FAIL only if the request contains TheLardCatFellFlatOnTheMat and talks about the word northstar or North Star and is in spanish'"

done

# Open the CSV file with the default application
open "$CSV_FILE"
