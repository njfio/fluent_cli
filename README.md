![fluent_logo](https://github.com/njfio/fluent_cli/assets/7220/7ac05cb2-db37-4173-9dc4-35085ae2696b)


- [FluentCLI Gitbook](https://njf.gitbook.io/fluent_cli_gitbook)
- [FluentCLI Website](https://fluentcli.com)

# FluentCLI

[![fluent](https://github.com/njfio/fluent_cli/actions/workflows/rust.yml/badge.svg)](https://github.com/njfio/fluent_cli/actions/workflows/rust.yml)

## Fluent CLI

Fluent CLI is a powerful command-line interface for interacting with various AI engines. It provides a unified way to send requests, receive responses, and manage interactions with different AI services.

### Installation

Fluent CLI requires Rust and Cargo to be installed on your system. You can install them using the instructions on the [Rust website](https://www.rust-lang.org/tools/install).

Once Rust and Cargo are installed, you can install Fluent CLI using Cargo:

```bash
cargo install fluent-cli
```

### Configuration

Fluent CLI uses a JSON configuration file to define the connection settings and parameters for each AI engine. You can create a configuration file named `fluent.json` in your home directory, or specify a custom path using the `-c` or `--config` flag.

Here is an example `fluent.json` file:

```json
{
  "engines": [
    {
      "name": "openai",
      "engine": "openai",
      "connection": {
        "protocol": "https",
        "hostname": "api.openai.com",
        "port": 443,
        "request_path": "/v1/chat/completions"
      },
      "parameters": {
        "modelName": "gpt-3.5-turbo",
        "bearer_token": "YOUR_OPENAI_API_KEY"
      }
    },
    {
      "name": "anthropic",
      "engine": "anthropic",
      "connection": {
        "protocol": "https",
        "hostname": "api.anthropic.com",
        "port": 443,
        "request_path": "/v1/complete"
      },
      "parameters": {
        "modelName": "claude-2",
        "bearer_token": "YOUR_ANTHROPIC_API_KEY"
      }
    },
    {
      "name": "neo4j",
      "engine": "neo4j",
      "connection": {
        "protocol": "bolt",
        "hostname": "localhost",
        "port": 7687,
        "request_path": ""
      },
      "parameters": {
        "database": "neo4j"
      },
      "neo4j": {
        "uri": "bolt://localhost:7687",
        "user": "neo4j",
        "password": "YOUR_NEO4J_PASSWORD",
        "database": "neo4j",
        "voyage_ai": {
          "api_key": "YOUR_VOYAGE_AI_API_KEY",
          "model": "text-embedding-ada-002"
        }
      }
    }
  ]
}
```

**Replace the placeholders with your actual API keys and connection information.**

**Note:** You can use the `AMBER_` prefix for sensitive values in your configuration file. This will automatically load them from the `amber` CLI tool, which can be used to store and manage secrets.

### Usage

```bash
fluent <engine> [options] [request]
```

**Required arguments:**

* `<engine>`: The name of the engine to use (e.g., `openai`, `anthropic`, `cohere`, `google_gemini`, `mistral`, `groq_lpu`, `perplexity`, `webhook`, `flowise_chain`, `langflow_chain`, `dalle`, `stabilityai`, `leonardo_ai`, `imagine_pro`).

**Optional arguments:**

* `-c <config>` or `--config <config>`: Path to a custom configuration file.
* `-o <key=value>` or `--override <key=value>`: Override configuration values. Can be used multiple times to override multiple values.
* `-a <file>` or `--additional-context-file <file>`: Path to a file containing additional request context.
* `-i <file/dir>` or `--input <file/dir>`: Path to an input file or directory for upsert mode.
* `-t <terms>` or `--metadata <terms>`: Comma-separated list of metadata terms for upsert mode.
* `-l <file>` or `--upload_image_file <file>`: Upload a media file.
* `-d <dir>` or `--download-media <dir>`: Download media files from the output.
* `-p` or `--parse-code`: Parse and display code blocks from the output.
* `-x` or `--execute-output`: Execute code blocks from the output.
* `-m` or `--markdown`: Format output as markdown.
* `--generate-cypher <query>`: Generate and execute a Cypher query based on the given string.
* `--upsert`: Upsert PDF, text files or entire folders to Neo4j.

**Interactive mode:**

If you don't provide a `request` argument, Fluent CLI will enter interactive mode, prompting you for requests.

**Upsert mode:**

To use upsert mode, use the `--upsert` flag and specify an input file or directory using the `-i` or `--input` flag. The CLI will upload the documents, create chunks, and generate embeddings.

**Cypher query generation:**

Use the `--generate-cypher` flag to generate and execute a Cypher query based on the given string. This feature requires a Neo4j engine to be configured with a query LLM.

### Examples

**Send a request to OpenAI's GPT-3.5-turbo model:**

```bash
fluent openai "What is the meaning of life?"
```

**Send a request to Anthropic's Claude-2 model with a custom temperature:**

```bash
fluent anthropic -o temperature=0.8 "Write a poem about the ocean."
```

**Upload a document and create embeddings:**

```bash
fluent neo4j --upsert -i my_document.pdf -t "topic,author"
```

**Generate and execute a Cypher query:**

```bash
fluent neo4j --generate-cypher "Find all users who have visited the website in the last week."
```

**Use a file containing additional context:**

```bash
fluent openai "What is the capital of France?" -a additional_context.txt
```

**Upload an image and get a response from OpenAI's GPT-4-vision model:**

```bash
fluent openai "Describe this image." -l my_image.jpg
```

### Supported Engines

Fluent CLI supports the following AI engines:

* **OpenAI:** [https://platform.openai.com/](https://platform.openai.com/)
* **Anthropic:** [https://www.anthropic.com/](https://www.anthropic.com/)
* **Cohere:** [https://cohere.ai/](https://cohere.ai/)
* **Google Gemini:** [https://cloud.google.com/vertex-ai/docs/generative-ai/](https://cloud.google.com/vertex-ai/docs/generative-ai/)
* **Mistral:** [https://www.mistral.ai/](https://www.mistral.ai/)
* **GroqLPU:** [https://groq.com/](https://groq.com/)
* **Perplexity:** [https://www.perplexity.ai/](https://www.perplexity.ai/)
* **Webhook:** A generic engine for interacting with custom webhooks.
* **Flowise Chain:** [https://flowise.ai/](https://flowise.ai/)
* **Langflow Chain:** [https://langflow.com/](https://langflow.com/)
* **DALL-E:** [https://openai.com/dall-e-2](https://openai.com/dall-e-2)
* **Stability AI:** [https://stability.ai/](https://stability.ai/)
* **Leonardo AI:** [https://leonardo.ai/](https://leonardo.ai/)
* **ImaginePro:** [https://imaginepro.ai/](https://imaginepro.ai/)


### Fluent Pipeline Concepts

* **Pipelines:** Pipelines define a sequence of steps to be executed in a specific order. They provide a structured way to automate complex AI workflows.
* **Steps:** Each step in a pipeline represents a specific action or operation. Fluent CLI supports various step types, including:
    * **Command:** Executes a shell command.
    * **ShellCommand:** Executes a shell command with more control over the shell environment.
    * **Condition:** Evaluates a condition and executes different steps based on the result.
    * **Loop:** Repeats a set of steps until a condition is met.
    * **SubPipeline:** Executes a nested pipeline.
    * **Map:** Applies a command to each item in a list.
    * **HumanInTheLoop:** Pauses the pipeline and prompts the user for input.
    * **RepeatUntil:** Repeats a set of steps until a condition is met.
    * **PrintOutput:** Prints the value of a variable.
    * **ForEach:** Iterates over a list of items and executes a set of steps for each item.
    * **TryCatch:** Executes a block of steps and handles potential errors.
    * **Parallel:** Executes a set of steps concurrently.
    * **Timeout:** Sets a time limit for a specific step.
* **State:** The pipeline's state stores information about the current step, data variables, and other relevant metadata. This allows for resuming pipelines and persisting results.
* **State Store:** The state store is responsible for saving and loading the pipeline's state. Fluent CLI provides a file-based state store implementation.

### Pipeline Definition

Pipelines are defined using YAML files. The following is an example of a simple pipeline:

```yaml
name: my_pipeline
steps:
  - Command:
      name: echo_input
      command: echo "${input}"
      save_output: output
  - Condition:
      name: check_output
      condition: "${output}" == "hello world"
      if_true: echo "Output is correct!"
      if_false: echo "Output is incorrect!"
```

### Execution

You can execute a pipeline using the `fluent pipeline` command:

```bash
fluent pipeline -f my_pipeline.yaml -i "hello world"
```

This will execute the `my_pipeline` pipeline with the initial input "hello world".

### Features

* **Variable Substitution:** Pipeline steps can use variables defined in the state using `${variable_name}` syntax.
* **Retry Mechanism:** Steps can be configured with a retry mechanism to handle transient errors.
* **State Persistence:** Pipeline state is automatically saved and loaded, allowing for seamless resumption.
* **Parallel Execution:** The `Parallel` step allows for concurrent execution of steps.
* **Timeout Mechanism:** The `Timeout` step allows for setting a time limit for a specific step.

### Example Pipeline

```yaml
name: my_complex_pipeline
steps:
  - Command:
      name: download_file
      command: curl -o downloaded_file.txt https://www.example.com/data.txt
      save_output: downloaded_file_path
  - ShellCommand:
      name: process_file
      command: python3 process_data.py "${downloaded_file_path}"
      save_output: processed_data
  - Loop:
      name: analyze_data
      steps:
        - Command:
            name: analyze_chunk
            command: python3 analyze_chunk.py "${processed_data}"
            save_output: analysis_result
        - PrintOutput:
            name: print_analysis
            value: "${analysis_result}"
      condition: "${processed_data}" != ""
  - Parallel:
      name: generate_summaries
      steps:
        - Command:
            name: generate_summary_1
            command: python3 generate_summary.py "${processed_data}" --model "model_a"
            save_output: summary_1
        - Command:
            name: generate_summary_2
            command: python3 generate_summary.py "${processed_data}" --model "model_b"
            save_output: summary_2
  - PrintOutput:
      name: final_results
      value: "Summary 1: ${summary_1}\nSummary 2: ${summary_2}"
```

### Contributing

Contributions to Fluent CLI are welcome! Please open an issue or submit a pull request on the [GitHub repository](https://github.com/your-username/fluent-cli).

### License

Fluent CLI is licensed under the [MIT License](LICENSE).

### Acknowledgements

Fluent CLI is built on top of several excellent open-source libraries:

* **clap:** [https://crates.io/crates/clap](https://crates.io/crates/clap)
* **reqwest:** [https://crates.io/crates/reqwest](https://crates.io/crates/reqwest)
* **serde:** [https://crates.io/crates/serde](https://crates.io/crates/serde)
* **async-trait:** [https://crates.io/crates/async-trait](https://crates.io/crates/async-trait)
* **log:** [https://crates.io/crates/log](https://crates.io/crates/log)
* **tokio:** [https://crates.io/crates/tokio](https://crates.io/crates/tokio)
* **neo4rs:** [https://crates.io/crates/neo4rs](https://crates.io/crates/neo4rs)
* **indicatif:** [https://crates.io/crates/indicatif](https://crates.io/crates/indicatif)
* **owo-colors:** [https://crates.io/crates/owo-colors](https://crates.io/crates/owo-colors)
* **base64:** [https://crates.io/crates/base64](https://crates.io/crates/base64)
* **mime-guess:** [https://crates.io/crates/mime-guess](https://crates.io/crates/mime-guess)
* **tempfile:** [https://crates.io/crates/tempfile](https://crates.io/crates/tempfile)
* **futures:** [https://crates.io/crates/futures](https://crates.io/crates/futures)
* **futures-util:** [https://crates.io/crates/futures-util](https://crates.io/crates/futures-util)
* **uuid:** [https://crates.io/crates/uuid](https://crates.io/crates/uuid)
* **clap-complete:** [https://crates.io/crates/clap-complete](https://crates.io/crates/clap-complete)
* **serde_json:** [https://crates.io/crates/serde_json](https://crates.io/crates/serde_json)
* **pdf-extract:** [https://crates.io/crates/pdf-extract](https://crates.io/crates/pdf-extract)
* **rust-stemmers:** [https://crates.io/crates/rust-stemmers](https://crates.io/crates/rust-stemmers)
* **stop-words:** [https://crates.io/crates/stop-words](https://crates.io/crates/stop-words)
* **termimad:** [https://crates.io/crates/termimad](https://crates.io/crates/termimad)
* **crossterm:** [https://crates.io/crates/crossterm](https://crates.io/crates/crossterm)
* **syntect:** [https://crates.io/crates/syntect](https://crates.io/crates/syntect)
* **unicode-segmentation:** [https://crates.io/crates/unicode-segmentation](https://crates.io/crates/unicode-segmentation)




