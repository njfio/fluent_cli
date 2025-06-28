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
* **Parallel Execution:** The `Parallel` step allows for concurrent execution of steps. Nested `Parallel` blocks are also run concurrently and their results aggregated when all tasks finish.
* **Timeout Mechanism:** The `Timeout` step allows for setting a time limit for a specific step.

### Example Pipeline

```yaml
name: ai_powered_content_creator_and_analyzer
steps:
  - !ShellCommand
    name: generate_article_topic
    command: |
      fluent openai '' <<EOT
      Generate a unique and intriguing article topic about the intersection of technology and nature
      Integrate this concept also: ${input}
      EOT
    save_output: article_topic

  - !ShellCommand
    name: create_article_outline
    command: |
      fluent sonnet3.5 '' <<"""EOT"""
      Create a detailed outline for an article with the following topic: ${article_topic}. Include exactly 5 main sections with 3-4 subsections each. Format the outline as a numbered list with main sections numbered 1-5 and subsections using letters (a, b, c, d).
      EOT
    save_output: article_outline

  - !ShellCommand
    name: initialize_article
    command: |
      echo "# ${article_topic}" > full_article.md
      echo "" >> full_article.md
      echo "${article_outline}" >> full_article.md
      echo "" >> full_article.md
    save_output: init_article

  - !ForEach
    name: generate_article_sections
    items: "1,2,3,4,5"
    steps:
      - !ShellCommand
        name: generate_section
        command: |
          fluent openai '' <<"""EOT"""
          Write a detailed section for the following part of the article outline: 

          ${article_outline}

          Focus on main section ${ITEM} and its subsections. Write approximately 500-700 words for this section. Include a section header formatted as an H2 (##) and subheaders as H3 (###).
          EOT
        save_output: current_section

      - !ShellCommand
        name: append_section
        command: |
          echo "${current_section}" >> full_article.md
          echo -e "\n\n" >> full_article.md
        save_output: append_result

  - !ShellCommand
    name: generate_introduction
    command: |
      fluent openai '' <<"""EOT"""
      Write an engaging introduction for the article with the following topic and outline:

      Topic: ${article_topic}
      Outline: ${article_outline}

      The introduction should be approximately 250-300 words and should set the stage for the rest of the article.
      EOT
    save_output: introduction

  - !ShellCommand
    name: generate_conclusion
    command: |
      fluent openai '' <<"""EOT"""
      Write a compelling conclusion for the article with the following topic and outline:

      Topic: ${article_topic}
      Outline: ${article_outline}

      The conclusion should be approximately 250-300 words, summarize the main points, and leave the reader with a final thought or call to action.
      EOT
    save_output: conclusion

  - !ShellCommand
    name: finalize_article
    command: |
      sed -i '1i\'"${introduction}"'' full_article.md
      echo -e "\n\n${conclusion}" >> full_article.md
    save_output: finalize_result

  - !ShellCommand
    name: read_full_article
    command: |
      cat full_article.md
    save_output: full_article

  - !ShellCommand
    name: web_scraping
    command: |
      PYTHON_CMD=$(which python || which python3)
      if [ -z "$PYTHON_CMD" ]; then
        echo "Python not found. Please install Python and ensure it's in your PATH."
        exit 1
      fi
      $PYTHON_CMD <<EOT
      import requests
      from bs4 import BeautifulSoup
      import json
      import re
      
      topic = """${article_topic}"""
      topic = re.sub(r'[^\w\s-]', '', topic).strip().replace(' ', '+')
      url = f'https://news.google.com/search?q={topic}&hl=en-US&gl=US&ceid=US:en'
      response = requests.get(url)
      soup = BeautifulSoup(response.text, 'html.parser')
      articles = soup.find_all('article')
      results = []
      for article in articles[:5]:
        title = article.find('h3')
        link = article.find('a')
        if title and link:
          results.append({
          'title': title.text,
          'link': 'https://news.google.com' + link['href'][1:]
        })
      print(json.dumps(results))
      EOT
    save_output: related_articles

  - !ShellCommand
    name: summarize_related_articles
    command: |
      fluent cohere '' <<"""EOT"""
      Summarize the following related articles in the context of our main topic '${article_topic}': ${related_articles}
      EOT
    save_output: article_summary

  - !ShellCommand
    name: generate_data_visualization
    command: |
      fluent gemini-flash ''  --parse-code <<"""EOT"""
      Create a Python script using matplotlib to visualize the relationship between technology and nature based on the article we've been working on. Use the following topic as inspiration: ${article_topic}
      No pip installs.  
      Only output the python script.  
      Save the plot as output.png
      EOT
    save_output: data_viz_script

  - !ShellCommand
    name: execute_data_visualization
    command: |
      python3 <<EOT
      ${data_viz_script}
      EOT
    save_output: data_viz_output

  - !ShellCommand
    name: generate_image_prompt
    command: |
      fluent sonnet3.5 '' <<"""EOT"""
      Create a detailed prompt for an AI image generator to create an image that represents the intersection of technology and nature, based on our article topic: 
      ${conclusion}
      Output the prompt only.
      EOT
    save_output: image_prompt

  - !Command
    name: generate_image
    command: |
      fluent dalleVertical '' --download-media ~/Downloads <<EOT
      "${image_prompt}"
      EOT
    save_output: generated_image

  - !ShellCommand
    name: analyze_generated_content
    command: |
      fluent mistral-nemo '' <<"""EOT"""
      Analyze the following content we've generated for our article. Provide insights on coherence, factual accuracy, and potential improvements:

      Topic: ${article_topic}
      Outline: ${article_outline}
      First Section: ${article_section_1}
      Related Articles Summary: ${article_summary}
      Generated Image Prompt: ${image_prompt}
      EOT
    save_output: content_analysis

  - !Command
    name: generate_social_media_post
    command: |
      fluent openai '' <<EOT
      Create an engaging social media post (280 characters max) to promote our article on the topic: ${article_topic}. 
      Include a call-to-action and relevant hashtags.
      EOT
    save_output: social_media_post

  - !ShellCommand
    name: create_markdown_report
    command: |
      fluent sonnet3.5 '' <<"""EOT"""
      Create a markdown report summarizing our content creation process and results. Include the following sections: Article Topic, Outline, Full Article Content, Related Articles Summary, Data Visualization Description, Generated Image Description, Content Analysis, and Social Media Post.
      EOT
    save_output: markdown_report

  - !ShellCommand
    name: save_report
    command: |
      cat <<EOT > ai_content_creator_report.md
      ${markdown_report}
      EOT
      echo "Report saved as ai_content_creator_report.md"


  - !PrintOutput
    name: print_summary
    value: |
      ======= AI-Powered Content Creator and Analyzer Pipeline Complete =======
      
      ${process_summary}
      
      You can find the full report in ai_content_creator_report.md
      The data visualization is saved as data_visualization.png
      The AI-generated image is saved in the output directory
      
      Thank you for using the AI-Powered Content Creator and Analyzer Pipeline!
      =======================================================================



  - !ShellCommand
    name: extract_triples
    command: |
      fluent openai-mini "give me an output of all the meaningful triples in this text. Only output the cypher in Neo4j format. use single quotes" --parse-code <<"""EOT"""
      ${final_summary}
      EOT
    save_output: triples_data

  - !ShellCommand
    name: add_triples
    command: |
      fluent neo4j --generate-cypher "create a cypher that adds these triples to the graph always do merge over create, ${triples_data}"
    save_output: add_triples_data

  - !ShellCommand
    name: final_summary
    command: |
      fluent openai '' <<EOT
      Summarize the entire process we've just completed in creating an AI-powered article, including the steps taken and the potential impact of this automated content creation pipeline.
      Only output the summary.
      EOT
    save_output: process_summary
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




