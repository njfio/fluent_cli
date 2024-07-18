#!/bin/bash

# Output file
output_file="source_compilation.txt"

# Clear the output file if it exists or create it if it doesn't
> "$output_file"

# Function to process files
process_files() {
    local dir="$1"
    find "$dir" -type f \( -name "*.rs" -o -name "*.toml" \) | while read -r file; do
        echo "#### START OF FILE: $file ####" >> "$output_file"
        cat "$file" >> "$output_file"
        echo -e "\n#### END OF FILE: $file ####\n" >> "$output_file"
    done
}

# Process the src/ and crates/ directories
process_files "src"
process_files "crates"

echo "All .rs and .toml files have been compiled into $output_file"
