
# Assuming FLUENT_CLI_CONFIG_PATH points to a JSON file containing configuration
autocomplete_flows() {
    local current_word="${COMP_WORDS[COMP_CWORD]}"
    local flow_names=$(jq -r '.[].name' "$FLUENT_CLI_CONFIG_PATH")
    COMPREPLY=($(compgen -W "${flow_names}" -- "$current_word"))
}
complete -F autocomplete_flows fluent

