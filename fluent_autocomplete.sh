#!/bin/bash

# Simple fuzzy match function
fuzzy_match() {
    local pattern="$1"
    local word="$2"
    local pattern_length=${#pattern}
    local word_length=${#word}

    # If the pattern is longer than the word, no match
    if [ "$pattern_length" -gt "$word_length" ]; then
        return 1
    fi

    local i=0
    local j=0
    while [ "$i" -lt "$pattern_length" ] && [ "$j" -lt "$word_length" ]; do
        if [ "${pattern:$i:1}" = "${word:$j:1}" ]; then
            i=$((i + 1))
        fi
        j=$((j + 1))
    done

    # If we've matched all characters in the pattern, it's a match
    [ "$i" -eq "$pattern_length" ]
}

# Fuzzy filter function
fuzzy_filter() {
    local cur="$1"
    shift
    local words=("$@")
    local matches=()

    for word in "${words[@]}"; do
        if fuzzy_match "$cur" "$word"; then
            matches+=("$word")
        fi
    done

    echo "${matches[@]}"
}

_fluent_cli_v2_autocomplete() {
    local cur prev words cword
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    words=("${COMP_WORDS[@]}")
    cword=$COMP_CWORD

    local config_file=""
    local selected_engine=""
    local request_entered=false

    # Determine if config is present and get its value
    for ((i=1; i<cword; i++)); do
        if [[ ${words[i]} == "-c" || ${words[i]} == "--config" ]]; then
            config_file="${words[i+1]}"
            break
        fi
    done

    if [[ -z "$config_file" ]]; then
        config_file="${FLUENT_CLI_V2_CONFIG_PATH:-}"
    fi

    # Function to parse JSON and extract values
    parse_json() {
        local file="$1"
        local engines=""

        if [[ -f "$file" ]]; then
            # Extract engine names
            engines=$(jq -r '.engines[].name' "$file" 2>/dev/null | tr '\n' ' ')
        fi

        echo "$engines"
    }

    local engines=""

    if [[ -f "$config_file" ]]; then
        engines=$(parse_json "$config_file")
    fi

    # Determine the selected engine and if request is entered
    for ((i=1; i<cword; i++)); do
        if [[ ${words[i]} != -* && ${words[i-1]} != "-c" && ${words[i-1]} != "--config" && ${words[i-1]} != "-a" && ${words[i-1]} != "--additional-context-file" ]]; then
            if [[ -z "$selected_engine" ]]; then
                selected_engine="${words[i]}"
            else
                request_entered=true
                break
            fi
        fi
    done

    case "${prev}" in
        -c|--config|-a|--additional-context-file)
            COMPREPLY=( $(compgen -f -- "${cur}") )
            return 0
            ;;
        --override|-o)
            if [[ -n "$selected_engine" && -f "$config_file" ]]; then
                local engine_parameters=$(jq -r ".engines[] | select(.name == \"$selected_engine\") | .parameters | keys[]" "$config_file" 2>/dev/null | sort -u | tr '\n' ' ')
                local filtered_params=$(fuzzy_filter "$cur" $engine_parameters)
                COMPREPLY=( $(compgen -W "${filtered_params}" -- "${cur}") )
                if [[ ${#COMPREPLY[@]} -eq 1 ]]; then
                    COMPREPLY=( "${COMPREPLY[0]}=" )
                fi
            fi
            return 0
            ;;
    esac

    # If we're at the very start, suggest only engines
    if [[ ${cword} -eq 1 ]]; then
        local filtered_engines=$(fuzzy_filter "$cur" $engines)
        COMPREPLY=( $(compgen -W "${filtered_engines}" -- "${cur}") )
        return 0
    fi

    # If we're at the first argument after config, suggest engines
    if [[ ${cword} -eq 3 && ${words[1]} == "--config" ]]; then
        local filtered_engines=$(fuzzy_filter "$cur" $engines)
        COMPREPLY=( $(compgen -W "${filtered_engines}" -- "${cur}") )
        return 0
    fi

    # If we're right after the engine selection, add quotes for the request
    if [[ -n "$selected_engine" && ${cword} -eq 2 && ${cur} == "" ]]; then
        COMPREPLY=( "\"\"" )
        # This will place the cursor inside the quotes in most terminals
        printf '\e[1;2D'
        return 0
    fi

    # If we're past the engine selection, suggest other options or nothing (for the request)
    if [[ -n "$selected_engine" ]]; then
        if [[ ${cur} == -* || $request_entered == true ]] ; then
            local opts="--override --upsert --input --metadata --upload_image_file --download-media --parse-code --execute-output --markdown --additional-context-file --generate-cypher"
            local filtered_opts=$(fuzzy_filter "$cur" $opts)
            COMPREPLY=( $(compgen -W "${filtered_opts}" -- "${cur}") )
        else
            # If it's not an option and request hasn't been entered, don't suggest anything
            COMPREPLY=()
        fi
        return 0
    fi
}

complete -o nospace -F _fluent_cli_v2_autocomplete fluent