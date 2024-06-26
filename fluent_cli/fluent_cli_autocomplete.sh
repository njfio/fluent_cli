# Assuming FLUENT_CLI_CONFIG_PATH points to a JSON file containing configuration
autocomplete_flows() {
    local i cur prev opts cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd=""
    opts=""

    # Define command options
    opts="-o -f -a -u -g -p -z -m -d -n -l -w -h -V --system-prompt-override-inline --system-prompt-override-file --additional-context-file --upload-file-path --generate-autocomplete --parse-code-output --full-output --markdown-output --download-media --upsert-no-upload --upsert-with-upload --webhook --help --version [flowname] [request] [context]"

    for i in "${COMP_WORDS[@]}"; do
        case "${cmd},${i}" in
            ",$1")
                cmd="fluent"
                ;;
            *)
                ;;
        esac
    done

    case "${cmd}" in
        fluent)
            # Add flow name autocomplete from JSON file
            if [[ ${cur} == *[a-zA-Z]* && ${prev} == "fluent" ]]; then
                local flow_names=$(jq -r '.[].name' "$FLUENT_CLI_CONFIG_PATH")
                COMPREPLY=($(compgen -W "${flow_names}" -- "${cur}"))
                return 0
            fi

            if [[ ${cur} == -* || ${COMP_CWORD} -eq 1 ]]; then
                COMPREPLY=( $(compgen -W "${opts}" -- "${cur}") )
                return 0
            fi

            case "${prev}" in
                --system-prompt-override-inline|--system-prompt-override-file|--additional-context-file|--upload-file-path|--download-media|--upsert-no-upload|--upsert-with-upload)
                    COMPREPLY=($(compgen -f -- "${cur}"))
                    return 0
                    ;;
            esac
            ;;
    esac
}

if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
    complete -F autocomplete_flows -o nosort -o bashdefault -o default fluent
else
    complete -F autocomplete_flows -o bashdefault -o default fluent
fi
