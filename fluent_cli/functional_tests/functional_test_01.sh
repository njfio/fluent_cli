#! /bin/bash

# These 2 commands will test the command and all input flags at the same time to ensure they work correctly.
# ls | RUST_LOG=debug /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli GroqMixtral8x7bAgentAnotherWebService "Describe the sky. And what are these?  " --System-Prompt-Override-Inline 'In Spanish Only'  --Additional-Context-File /Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests/functional_test_outline.txt
# ls | RUST_LOG=debug /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli GroqMixtral8x7bAgentAnotherWebService "Describe the sky. And what are these?  " --System-Prompt-Override-File /Users/n/Downloads/PreviousDaysDownloads/functional_tests/functional_test_spanish_system_prompt.txt  --Additional-Context-File /Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests/functional_test_outline.txt


CSV_FILE="/Users/n/Downloads/functional_test_results.csv"
echo "FlowName,Question,Result" > "$CSV_FILE"

alias fluent_cli=/Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli

FLOWNAME=GPT4FunctionAgentWithMemoryAndBrowsing
IMAGEFLOWNAME=ImageGeneratorRealvisxlRepoCloud
SYSTEM_PROMPT_FILE=/Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests/functional_test_spanish_system_prompt.txt
OUTLINE_FILE=/Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests/functional_test_outline.txt
CONTEXT_FILE=/Users/n/RustroverProjects/fluent_cli/fluent_cli/functional_tests/functional_test_context.txt
IMAGE_FILE_PATH=/Users/n/Downloads/


echo "RUNNING IS THE fluent (FLOWISE LANGUAGE UTILITY for ENHANCED NETWORKED TRANSACTIONS) FUNCTIONAL TEST SCRIPT"
echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 0    Evaluating base fluent_cli <flowname> this is a test"
echo "================================================================================================="
echo ""

TEST_OUTPUT=$(  /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "This is a test, respond that this is a test" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the answer is about this is a test")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 0  $RESULT    Tested base fluent_cli <flowname> this is a test"
echo "================================================================================================="

echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"


echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 1     Testing <stdin> context"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$(cat "$CONTEXT_FILE" |  /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "This is the content: " | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the context has the word northstar")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 1  $RESULT    Tested <stdin> context"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"


echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 2     Testing --Additional-Context-File"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$( /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "What is the content of this file" --Additional-Context-File "$OUTLINE_FILE" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the context contains the word 'TheLardCatFellFlatOnTheMat'")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 2  $RESULT    Tested --Additional-Context-File"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"

echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 3     Testing <stdin> context and --Additional-Context-File"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$( cat "$CONTEXT_FILE" | /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "What are these contents about:" --Additional-Context-File "$OUTLINE_FILE" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the context contains the word 'TheLardCatFellFlatOnTheMat' and contains the word 'northstar'")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 3  $RESULT   Tested <stdin> context and --Additional-Context-File"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"


echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 4     Testing <stdin> context and --System-Prompt-Override-Inline and --Additional-Context-File"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$(  /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "Summarize these details:" --System-Prompt-Override-Inline "You can only reply in german" --Additional-Context-File "$OUTLINE_FILE" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the answer is in german and contain either, TheLardCatFellFlatOnTheMat or ChickenLickenIsQuicklyTicken")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 4  $RESULT  Tested <stdin> context and --System-Prompt-Override-Inline and --Additional-Context-File"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"



echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 5     Testing <stdin> context and --System-Prompt-Override-Inline and --Additional-Context-File"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$( cat "$CONTEXT_FILE" | /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "Summarize these details:" --System-Prompt-Override-Inline "You can only reply in german" --Additional-Context-File "$OUTLINE_FILE" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the answer is in german and contain either, TheLardCatFellFlatOnTheMat or ChickenLickenIsQuicklyTicken and something about the northstar")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 5  $RESULT    Tested <stdin> context and --System-Prompt-Override-Inline and --Additional-Context-File"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"




echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 6     Testing --System-Prompt-Override-Inline"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$(  /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "Tell me about the wizard of oz." --System-Prompt-Override-Inline "You can only reply in german" | tee /dev/tty  | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the answer is in german")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 6  $RESULT    Tested --System-Prompt-Override-Inline"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"



echo ""
echo ""
echo ""
echo ""
echo "================================================================================================="
echo " 7     Testing --System-Prompt-Override-File"
echo "-------------------------------------------------------------------------------------------------"
echo ""

TEST_OUTPUT=$(  /Users/n/RustroverProjects/fluent_cli/fluent_cli/target/release/fluent_cli "$FLOWNAME" "Tell me about the wizard of oz" --System-Prompt-Override-File "$SYSTEM_PROMPT_FILE" | tee  /dev/tty | fluent HaikuToolAgentRepoCloud "Answer PASS or FAIL only if the answer is in spanish")
echo "$TEST_OUTPUT"
RESULT=$(echo "$TEST_OUTPUT" | grep -oE "PASS|FAIL")
echo "-------------------------------------------------------------------------------------------------"
echo " 7  $RESULT    Tested --System-Prompt-Override-File"
echo "================================================================================================="
echo "$FLOWNAME,#1 context <stdin>,$RESULT" >> "$CSV_FILE"





open $CSV_FILE