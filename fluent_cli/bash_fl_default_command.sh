
fun_fl() {
  # Join all arguments into a single string, properly escaping internal quotes
  local input="$*"

  # Use printf to escape special characters safely
  printf -v escaped_input "%q" "$input"

  # Replace the escaping done by %q which adds unnecessary backslashes in front of spaces
  escaped_input="${escaped_input//\\ / }"

  # Execute the command with the cleaned up and escaped string
  fluent GroqLLama370b8192AgentRepoCloud "$escaped_input"
}

alias fl='fun_fl'