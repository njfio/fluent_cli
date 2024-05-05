#!/bin/bash

# Set default configuration file path
CONFIG_FILE="script_config.conf"

# Check for command line argument for configuration file
if [[ "$1" == "-c" ]] && [ "$#" -ge 3 ]; then
    CONFIG_FILE="$2"
    shift 2
elif [ "$#" -lt 2 ]; then
    echo "Usage: $0 [-c config_file_path] '<initial_request>'"
    exit 1
fi

# Load configuration file
if [ -f "$CONFIG_FILE" ]; then
    source "$CONFIG_FILE"
else
    echo "Configuration file not found at $CONFIG_FILE. Exiting."
    exit 1
fi

# Check for initial request
if [ -z "$1" ]; then
    echo "No initial request provided. Exiting."
    exit 1
fi
INITIAL_REQUEST="$1"

# Define functions
call_fluent() {
    local flow_name="$1"
    local input_request="$2"
    local output_file="$3"

    echo "$input_request" | fluent "$flow_name" ' ' > "$output_file"
    if [ $? -ne 0 ]; then
        echo "Failed to process $flow_name"
        exit 1
    fi
    echo -e "$flow_name output has been saved to $output_file\n"
}

verify_step() {
    local file="$1"
    local step="$2"

    echo -e "Review the content of $file for the $step step:\n"
    echo -e "\t\t---------------------------------------------------------------------------------\n\n"
    mdcat "$file"  --ansi # 'plain' removes git integration and headers
    echo -e "\n"
    echo -e "\n"
    echo -e "\t\t---------------------------------------------------------------------------------\n"
    read -p "Is this $step correct? (yes/no) " approval
    if [ "$approval" != "yes" ]; then
        echo "Error: $step approval failed. Exiting."
        exit 1
    fi
    echo "$step approved."
}

# Define file paths
STORY_ARC_FILE="$STORY_ARC_PATH"
CHARACTER_MAP_FILE="$CHARACTER_MAP_PATH"
OUTLINE_FILE="$OUTLINE_PATH"
PROMPTS_FILE="$PROMPTS_PATH"
FINAL_POST_FILE="$FINAL_POST_PATH"

# Generate story arc
call_fluent "$FLOW_STORY_ARC" "In paragraph form create a story arc about the request with innovative plot twists.  The request: $INITIAL_REQUEST" "$STORY_ARC_FILE"
verify_step "$STORY_ARC_FILE" "story arc"

# Generate character map
cat "$STORY_ARC_FILE" | fluent "$FLOW_CHARACTER_MAP" "Generate a complete creative character map for this story arc context.  Output just the character map." > "$CHARACTER_MAP_FILE"

verify_step "$CHARACTER_MAP_FILE" "character map"

# Generate outline
cat "$CHARACTER_MAP_FILE" "$STORY_ARC_FILE" | fluent "$FLOW_OUTLINE" "Create an extensive creative detailed outline for this story and provided context. Output just the outline." > "$OUTLINE_FILE"

verify_step "$OUTLINE_FILE" "outline"

# Generate prompts
cat "$CHARACTER_MAP_FILE" "$STORY_ARC_FILE" "$OUTLINE_FILE" | fluent "$FLOW_PROMPTS" "Generate 20-40 prompts that will tell this story eloquently with creativity, originality, and incredible suspension of disbelief.  Output just the prompts sequentially without numbering them" > "$PROMPTS_FILE"
if [ ! -s "$PROMPTS_FILE" ]; then
    echo "No prompts were generated or there was an error. Exiting."
    exit 1
fi

verify_step "$PROMPTS_FILE" "prompts"

# Ask for final approval to build the complete story
read -p "Generate the complete blog post based on the approved items? (yes/no) " final_approval
if [ "$final_approval" != "yes" ]; then
    echo "Final approval not given. Exiting."
    exit 1
fi

# Process each prompt to generate the story
: > "$FINAL_POST_FILE" # Clear existing content if any
while IFS= read -r prompt; do
    if [[ -n "$prompt" && "$prompt" =~ [^[:space:]] ]]; then
        {
            cat "$CHARACTER_MAP_FILE" "$STORY_ARC_FILE"
            tail -n 10 "$FINAL_POST_FILE"
        } | fluent "$FLOW_POST_SECTION" "Generating section based on context prompt.  No Yapping. Do not offer unnatural introductions or lead ins.  Do not summarize.  Just write the section.  You have access to the previous 15 lines in the context.  Transition eloquently.  Never lead-in or start 'Here is my attempt...' or similar. Eloquent transitions only. Use Markdown" >> "$FINAL_POST_FILE"

        {
           cat  $CHARACTER_MAP_FILE
           tail -n 10 "$FINAL_POST_FILE"
        } | fluent HaikuChain "Create a prompt that will be used to create an image for this context. The image should be in abstract art style.  Keep consistent with the provided character map and you align the theme with the provided last 15 lines in the context" | fluent MakeLeonardoImagePost "" -d /Users/n/Downloads/  >> "$FINAL_POST_FILE"
    else
        echo "Skipped empty or whitespace-only prompt."
    fi
done < "$PROMPTS_FILE"

echo "Blog post generation completed. Check '$FINAL_POST_FILE' for the full content."
pandoc "$FINAL_POST_FILE" --from markdown --to html5 --standalone --toc --highlight-style=espresso --output "$FINAL_POST_FILE.html"
cat "$FINAL_POST_FILE.html" | fluent MakeShopifyAndGhostPostExample "Blog"
cat "$FINAL_POST_FILE" | fluent GPT4FunctionAgentWithMemoryAndBrowsingRepoCloud "grade this blog post"