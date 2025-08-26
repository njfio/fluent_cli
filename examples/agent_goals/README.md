Examples: Agent Goals

How to use
- Copy `goal_description` into your CLI invocation (or integrate goal-file loading).
- Ensure tools are enabled and, optionally, MCP servers are configured in your config file.

Auto-connecting MCP
- In your main config (e.g., `config.yaml`), add:

```
mcp:
  servers:
    - name: search
      command: my-mcp-search-server
      args: ["--stdio"]
    - "browser:my-mcp-browser --stdio"
```

Research Goal
- File: `research_goal.toml`
- Fields: goal_description, max_iterations, output_dir
- Outputs under `output_dir`: `outline.md`, `notes.md`, `summary.md`.

Long-form Writing Goal
- File: `longform_goal.toml`
- Fields: goal_description, max_iterations, output_dir, chapters
- Outputs under `output_dir`: `outline.md`, `ch_01.md..`, `index.md`, `toc.md`, `book.md`.
