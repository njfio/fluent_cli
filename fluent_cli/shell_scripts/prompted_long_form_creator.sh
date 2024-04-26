#!/bin/bash

# Check for command line argument for configuration file
if [[ "$1" == "-c" ]] && [ "$#" -ge 3 ]; then
    config_file="$2"
    shift 2  # Adjust the position of positional parameters after removing config parameters
elif [ "$#" -lt 2 ]; then
    echo "Usage: $0 [-c config_file_path] '<initial_request>'"
    exit 1
fi

# Source the configuration file
if [ -f "$config_file" ]; then
    source "$config_file"
else
    echo "Configuration file not found at $config_file. Exiting."
    exit 1
fi

if [ -z "$1" ]; then
    echo "No initial request provided. Exiting."
    exit 1
fi
initial_request="$1"

call_fluent() {
    local flow_name=$1
    local input_request=$2
    local output_file=$3

    echo "$input_request" | fluent "$flow_name" ' ' > "$output_file" 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "Failed to process $flow_name"
        exit 1
    fi
    echo "$flow_name output has been saved to $output_file"
}

verify_step() {
    local file=$1
    local step=$2
    cat "$file"
    read -p "Is this $step correct? (yes/no) " approval
    if [ "$approval" != "yes" ]; then
        echo "$step approval failed. Exiting."
        exit 1
    fi
}

# Generate story arc
story_arc_file="$STORY_ARC_PATH"
call_fluent "$FLOW_STORY_ARC" "In paragraph form create a story arc about the request with innovative plot twists.  The request: $initial_request" "$story_arc_file"
verify_step "$story_arc_file" "story arc"

# Generate character map
character_map_file="$CHARACTER_MAP_PATH"
cat "$story_arc_file" | fluent "$FLOW_CHARACTER_MAP" "Generate a complete creative character map for this story arc context.  Output just the character map" > "$character_map_file"
echo character_map_file

verify_step "$character_map_file" "character map"

# Generate outline
outline_file="$OUTLINE_PATH"
cat "$character_map_file" "$story_arc_file" | fluent "$FLOW_OUTLINE" "Create an extensive creative detailed outline for this story and provided context.  output just the outline." > "$outline_file"
echo outline_file
verify_step "$outline_file" "outline"

# Generate prompts
prompts_file="$PROMPTS_PATH"
cat "$character_map_file" "$story_arc_file" "$outline_file" | fluent "$FLOW_PROMPTS" "Generate at least 20-40 prompts that will tell this story eloquently with creativity, originality, and incredible suspension of disbelief.  Output just the prompts sequentially without numbering them" > "$prompts_file"
if [ ! -s "$prompts_file" ]; then
    echo "No prompts were generated or there was an error. Exiting."
    exit 1
fi

# Ask for final approval to build the complete story
read -p "Generate the complete blog post based on the approved items? (yes/no) " final_approval
if [ "$final_approval" != "yes" ]; then
    echo "Final approval not given. Exiting."
    exit 1
fi

# Process each prompt to generate the story
final_post_file="$FINAL_POST_PATH"
: > "$final_post_file" # Clear existing content if any
while IFS= read -r prompt; do
    if [[ -n "$prompt" && "$prompt" =~ [^[:space:]] ]]; then  # Check if prompt is not empty and not just whitespace
        echo "Processing prompt: $prompt"
        {
            cat "$character_map_file" "$story_arc_file"
            tail -n 10 "$final_post_file"
        } | fluent "$FLOW_POST_SECTION" " Generating section based on context prompt.  No Yapping. Do not offer unnatural introductions or lead ins.  Do not summarize.  Just write the section.  You have access to the previous 10 lines in the context.  Transition eloquently.  Never lead-in or start 'Here is my attempt...' or similar. Eloquent transitions only. Use Markdown " >> "$final_post_file"

        {
           cat  $character_map_file
           tail -n 10 "$final_post_file"
        } | fluent SonnetChain " Create a prompt that will be used to create an image for this context. The image should be in abstract art style.  Keep consistent with the provided character map and you align the theme with the provided last 10 lines in the context " | fluent MakeLeonardoImagePost " " -d /Users/n/Downloads/  >> "$final_post_file"
    else
        echo "Skipped empty or whitespace-only prompt."
    fi
done < "$prompts_file"

echo "Blog post generation completed. Check '$final_post_file' for the full content."
pandoc "$final_post_file" --from markdown --to html5 --standalone --toc --highlight-style=espresso --output "$final_post_file.html"
cat "$final_post_file.html" | fluent MakeShopifyAndGhostPostExample "Blog"
cat "$final_post_file" | fluent OpusChain "grade this blog post"
