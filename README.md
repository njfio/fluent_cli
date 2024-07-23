![fluent_logo](https://github.com/njfio/fluent_cli/assets/7220/7ac05cb2-db37-4173-9dc4-35085ae2696b)

> This project is under very active development.
> 
> The documentation is *not* as up-to-date as I would like but it will be soon.
>
> I wanted to focus on the refinement of document generation at this stage.
>
> Please reach out directly if you have any questions.
> Docs will be updated soon covering all new functionality.  

- [FluentCLI Gitbook](https://njf.gitbook.io/fluent_cli_gitbook)
- [FluentCLI Website](https://fluentcli.com)

# FluentCLI

[![fluent](https://github.com/njfio/fluent_cli/actions/workflows/rust.yml/badge.svg)](https://github.com/njfio/fluent_cli/actions/workflows/rust.yml)

Fluent CLI is a command-line interface (CLI) tool designed to simplify and empower interaction with various AI LLMs and workflow engines including: OpenAI, Anthropic, Perplexity, Gemini, Groq, Cohere, FlowiseAI, and Langflow.  There is native API integration with StabilityAI, Dalle, LeonardoAI, and custom webhooks.  FluentCLI provides a streamlined way to send requests, manage configurations, handle responses, and chain commands in amazing new ways.

## Features

- **Multi-Engine Support:**  Interact with FlowiseAI, Langflow, and Webhook workflows seamlessly.
- **Native API Support:** OpenAI Assistants, OpenAI Agents, Dall-e, StabilieyAI, LeonardoAI, Anthropic, GroqLPU, Cohere, Google Studio, Cohere, and Perplexity models.
- **Simplified Request Handling:** Send requests to your workflows with a simple command structure.
- **Context Management:** Provide additional context via stdin or files for richer interactions.
- **Configuration Management:** Load and modify workflow configurations from a centralized JSON file.  Override inline. 
- **Environment Variable Integration:** Securely store sensitive information like API keys in environment variables and reference them in your configurations.
- **Override Inline:**  Easily override configuration parameters inline.
- **File Upload:**  Upload images and other files to your workflows.
- **Upsert Functionality:**  Send JSON payloads and upload files to endpoints for data management.
- **Output Customization:** Control output format, including stylized markdown, parsed code blocks, and full JSON responses.
- **Media Download:** Download media files linked in responses directly from the CLI.
- **Autocomplete Generation:** Generate Bash and Fig autocomplete scripts for enhanced usability.
- **Amber Integration:** Securely decrypt and manage sensitive keys using the Amber secrets management tool.
  
- **Neo4J Integration:** Seamless integration with Neo4j for efficient graph based data storage and retrieval.
- **Neo4j Natural Language Cypher Creation:** Let AI write your cyphers for you based on natural language input.
- **Neo4j Upserts:** Supporting Docx, PDF, Txt files and whole folders processed inline with embeddings created from VoyageAI to create nodes for similarity, sentiment, keywords, themes, clusters.

## FluentCLI Utility Ecosystem
- [Rust Fluent Code Utility Repository](https://github.com/njfio/rfcu)
- [Rust Airtable Utility Repository](https://github.com/njfio/rau)
- [Rust Logseq Utility Repository](https://github.com/njfio/rlu)

## Installation

### From Source

1. Ensure you have Rust and Cargo installed.
2. Clone this repository: `git clone https://github.com/njfio/fluent_cli.git`
3. Navigate to the project directory: `cd fluent_cli`
4. Build the project: `cargo build --release`
5. The executable will be located in `target/release/fluent`.

### Pre-built Binaries

Pre-built binaries for various platforms are available in the [Releases](https://github.com/njfio/fluent_cli/releases) section of this repository.

## Configuration

Fluent CLI uses a `config.json` file to store workflow configurations. A sample configuration file is provided in the repository. You can customize this file to include your own workflows and settings.

### Structure

```json
[
  {
    "name": "FlowName",
    "engine": "flowise|langflow|webhook",
    "protocol": "http|https",
    "hostname": "your-hostname",
    "port": 80|443,
    "chat_id": "your-chat-id",
    "request_path": "/api/v1/prediction/",
    "upsert_path": "/api/v1/vector/upsert/", // Optional, for upsert functionality
    "sessionId": "your-session-id",
    "bearer_token": "your-bearer-token",
    "overrideConfig": {}, // Workflow-specific overrides
    "tweaks": {}, // Workflow-specific tweaks
    "timeout_ms": 50000 
  },
  // ... more workflows
]
```

**Explanation:**

- **name:** A unique name for your workflow.
- **engine:** The type of workflow engine (flowise, langflow, webhook).
- **protocol, hostname, port, chat_id, request_path:**  Connection details for your workflow.
- **upsert_path:** (Optional) The path for upsert requests.
- **sessionId:** The session ID for your workflow (if applicable).
- **bearer_token:** The authentication token for your workflow.
- **overrideConfig:** A JSON object containing workflow-specific configuration overrides.
- **tweaks:** A JSON object containing workflow-specific tweaks.
- **timeout_ms:** The request timeout in milliseconds.

**Notes:**

- You can use environment variables in your configuration by prefixing the variable name with `AMBER_`. For example, to use the environment variable `MY_API_KEY` in your bearer token, set `bearer_token` to `AMBER_MY_API_KEY`.
- Fluent CLI will automatically attempt to decrypt keys starting with `AMBER_` using the Amber secrets management tool.

## Usage

### Basic Requests

To send a request to a workflow, use the following command:

```bash
fluent <flowname> "<request>"
```

**Example:**

```bash
fluent MyFlow "What is the weather like today?"
```

### Context

You can provide additional context to your request through stdin or a file:

**Stdin:**

```bash
echo "Here is some context." | fluent MyFlow "What is the weather like today?"
```

**File:**

```bash
fluent MyFlow "What is the weather like today?" --context "context.txt"
```

### System Prompt Override

To override the system message of a Flowise workflow, use the following options:

**Inline:**

```bash
fluent MyFlow "What is the weather like today?" --system-prompt-override-inline "You are a helpful weather bot."
```

**File:**

```bash
fluent MyFlow "What is the weather like today?" --system-prompt-override-file "system_prompt.txt"
```

### File Upload

To upload an image to a workflow, use the `--upload-image-path` option:

```bash
fluent MyFlow "Describe this image." --upload-image-path "image.png"
```

### Upsert

**With Upload:**

```bash
fluent MyFlow --upsert-with-upload "file1.txt,file2.csv"
```

**Without Upload:**

```bash
fluent MyFlow --upsert-no-upload
```

### Output Customization

**Markdown:**

```bash
fluent MyFlow "What is the weather like today?" --markdown-output
```

**Parsed Code Blocks:**

```bash
fluent MyFlow "Generate some Python code." --parse-code-output
```

**Full JSON Output:**

```bash
fluent MyFlow "What is the weather like today?" --full-output
```

### Media Download

```bash
fluent MyFlow "Find me some images of cats." --download-media "/path/to/directory"
```

### Autocomplete Generation

**Bash:**

```bash
fluent --generate-autocomplete > fluent_autocomplete.sh
source fluent_autocomplete.sh
```

**Fig:**

```bash
fluent --generate-fig-autocomplete > fluent.ts
```

### Override Configuration Values

```bash
fluent MyFlow "What is the weather like today?" --override modelName="gpt-4" --override tweaks.Prompt-PbKIE.template="You are a helpful pirate."
```

## Contributing

Contributions to Fluent CLI are welcome! Please open an issue or submit a pull request if you have any suggestions, bug reports, or feature requests.

## License

Fluent CLI is licensed under the MIT License.



Nicholas Ferguson - nick@njf.io

Project Link: [https://github.com/njfio/fluent-cli](https://github.com/njfio/fluent-cli)


