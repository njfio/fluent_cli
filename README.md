
## Fluent CLI: Streamline Your Workflows with Precision and Ease

**Fluent CLI** is an advanced command-line interface designed to interact seamlessly with multiple workflow systems like FlowiseAI, Make, and Zapier. Tailored for developers, IT professionals, and power-users, Fluent CLI facilitates robust automation, simplifies complex interactions, and enhances productivity through a powerful and configurable command suite.

### Key Features:

- **Multi-Service Integration**: Connect effortlessly with services like FlowiseAI, Make, and Zapier to automate and manage workflows across different platforms.

- **Dynamic Configuration**: Utilize JSON-based configurations to dynamically adjust command parameters, making your workflows flexible and adaptable to changing needs.

- **Enhanced File Handling**: Support for uploading images and files directly through the CLI, integrated smoothly with asynchronous operations to boost performance.

- **Secure Environment Interaction**: Automatic handling of environment variables and secure token management ensures that your operations are safe and your data is protected.

- **Versatile Output Options**: Whether you need beautifully formatted markdown, concise code blocks, or well-structured JSON, Fluent CLI delivers your data in the format you prefer, right in your terminal.- **Interactive Inputs**: Fluent CLI handles stdin inputs gracefully, allowing for interactive user sessions and seamless piping to and from other commands.

- **Autocomplete Workflow Names**: Fluent CLI includes autocomplete for all the configured workflow names which makes calling any of the workflows just a few keystrokes away.

- **Versionable Secure Vault**: Fluent CLI is integrated with [amber](https://github.com/fpco/amber), as a secure vault for configuration information.  Store your keys once and never worry about them again.  

- **Cross Platform Support**:  Written in Rust and works on Linux, Windows, and Macos.  Configure once, use everywhere.  


### Designed For:

- **Developers and System Administrators** who require a reliable tool to manage and automate workflows across various platforms.

- **DevOps Teams** looking for a versatile tool to integrate into continuous integration and deployment pipelines.

- **Tech Enthusiasts** and professionals who benefit from streamlined command-line tools to enhance their operational efficiency.


Fluent CLI is more than just a tool—it's your next step towards efficient and scalable workflow management. Jumpstart your automation with Fluent CLI today!

---


## Installation

To set up Fluent CLI on your local system, follow these steps:

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/fluent-cli.git
   ```
   
3. Navigate into the project directory:
   ```bash
   cd fluent-cli
   ```
   
4. Build the project using Cargo (Rust's package manager):
   ```bash
   cargo build --release
   ```

   ## Or Install Directly
   ```bash
   cargo install --git https://github.com/njfio/fluent_cli.git
   ```

## Configuration

If you have never used amber before, we need to start with it.  If you use amber just add the keys to your existing vault. 

Running `amber init` will create an `amber.yaml` file in the current directory and output your onetime private key.  

Do not lose this file or you will be unable to unlock the amber secrets file and run fluent.

> ## Securely store that secret key, such as in a password manager. Additionally, if desired, put that secret key in your CI system's secrets.

```bash

amber init                                                                                      
Your secret key is: 9d75ea642ed27900624b30de2e0f5ed979104d098918db92e50a9aa6f2a57952
Please save this key immediately! If you lose it, you will lose access to your secrets.
Recommendation: keep it in a password manager

If you're using this for CI, please update your CI configuration with a secret environment variable
export AMBER_SECRET=9d75ea642ed27900624b30de2e0f5ed979104d098918db92e50a9aa6f2a57952

```

## Adding keys to amber

Below is a starter table of keys for the included configs with fluent_cli.  It also includes links to get all API keys from the various services. 

It's incredibly easy to use.  Just get your api key from whatever service and run the command in the amber command column replacing the <content> with your key.  

The `AMBER_ANOTHERWEBSERVICE_NJF` example includes the bearer token for the service.   

Validate the keys you've entered by typing `amber print`



| Variable Name                         | Description                                        | API Key URL                                          | Amber Command                                               |
|---------------------------------------|----------------------------------------------------|------------------------------------------------------|-------------------------------------------------------------|
| `AMBER_FLUENT_SESSION_ID_01`          | Universal ID across your workflows                 |                                                      | `amber encrypt AMBER_FLUENT_SESSION_ID_01 <content>`        |
| `AMBER_ANOTHERWEBSERVICE_NJF`         | Bearer token for Flowise                           |                                                      | `amber encrypt AMBER_ANOTHERWEBSERVICE_NJF NUd1MEQ+w5VZDpoeBcFOihPe8VT5EY/vsbnZ8HfPit4=`       |
| `AMBER_LOCAL_FLUENT_DEFAULT_KEY`      | Bearer token for a local Flowise install           |                                                      | `amber encrypt AMBER_LOCAL_FLUENT_DEFAULT_KEY <content>`    |
| **LLM API KEYS**                      |                                                    |                                                      |                                                             |
| `AMBER_FLUENT_ANTHROPIC_KEY_01`       |                                                    | [Anthropic](https://console.anthropic.com/settings/keys) | `amber encrypt AMBER_FLUENT_ANTHROPIC_KEY_01 <content>`     |
| `AMBER_FLUENT_GROQ_API_KEY_01`        |                                                    | [GroqLPU](https://console.groq.com/keys)             | `amber encrypt AMBER_FLUENT_GROQ_API_KEY_01 <content>`      |
| `AMBER_FLUENT_MISTRAL_KEY_01`         |                                                    | [Mistral](https://console.mistral.ai/api-keys/)      | `amber encrypt AMBER_FLUENT_MISTRAL_KEY_01 <content>`       |
| `AMBER_FLUENT_OPENAI_API_KEY_01`      |                                                    | [OpenAI](https://platform.openai.com/api-keys)       | `amber encrypt AMBER_FLUENT_OPENAI_API_KEY_01 <content>`    |
| `AMBER_FLUENT_PERPLEXITY_API_KEY_01`  |                                                    | [Perplexity](https://www.perplexity.ai/settings/api) | `amber encrypt AMBER_FLUENT_PERPLEXITY_API_KEY_01 <content>`|
| `AMBER_FLUENT_GEMINI_API_KEY_01`      |                                                    | [Gemini](https://ai.google.dev/)                     | `amber encrypt AMBER_FLUENT_GEMINI_API_KEY_01 <content>`    |
| `AMBER_FLUENT_COHERE_API_KEY_01`      |                                                    | [Cohere](https://dashboard.cohere.com/api-keys)      | `amber encrypt AMBER_FLUENT_COHERE_API_KEY_01 <content>`    |
| `AMBER_FLUENT_HUGGINGFACE_API_KEY_01` |                                                    | [HuggingFace](https://huggingface.co/settings/tokens)| `amber encrypt AMBER_FLUENT_HUGGINGFACE_API_KEY_01 <content>`|
| `AMBER_FLUENT_REPLICATE_API_KEY_01`   |                                                    | [Replicate](https://replicate.com/account/api-tokens)| `amber encrypt AMBER_FLUENT_REPLICATE_API_KEY_01 <content>` |
| `AMBER_FLUENT_PINECONE_API_KEY_01`    |                                                    | [Pinecone](https://app.pinecone.io/...)              | `amber encrypt AMBER_FLUENT_PINECONE_API_KEY_01 <content>`  |
| `AMBER_FLUENT_SEARCHAPI_KEY_ID_01`    |                                                    | [SearchAPI](https://www.searchapi.io/)               | `amber encrypt AMBER_FLUENT_SEARCHAPI_KEY_ID_01 <content>`  |
| `AMBER_FLUENT_SERPAPI_KEY_01`         |                                                    | [SerpAPI](https://serpapi.com/manage-api-key)        | `amber encrypt AMBER_FLUENT_SERPAPI_KEY_01 <content>`       |
| `AMBER_FLUENT_ZEP_MEMORY_KEY_01`      |                                                    | [ZepMemory](https://app.getzep.com/projects/)        | `amber encrypt AMBER_FLUENT_ZEP_MEMORY_KEY_01 <content>`    |
| `AMBER_LEONARDO_AI_KINO_XL_MODEL_ID`  |                                                    |                                                      | `amber encrypt AMBER_LEONARDO_AI_KINO_XL_MODEL_ID <content>`|
| `AMBER_MAKE_LEONARDO_IMAGE_POST`      |                                                    |                                                      | `amber encrypt AMBER_MAKE_LEONARDO_IMAGE_POST <content>`    |


FluentCLI relies on a JSON configuration file to manage the workflow specifics. Ensure that the `FLUENT_CLI_CONFIG_PATH` environment variable is set to point to your configuration file.  

Use the included configuration file, `config.json` to start.  

Below is an example chatflow configuration in `config.json`.  
```json
  {
    "name": "SonnetXMLAgentAnowtherWebService",
    "protocol": "https",
    "hostname": "container-qygwpcc.containers.anotherwebservice.com",
    "port": 3000,
    "chat_id": "8ea46fa9-4aef-4184-a399-c588c576d148",
    "request_path": "/api/v1/prediction/",
    "sessionId": "",
    "bearer_token": "AMBER_ANOTHERWEBSERVICE_NJF",
    "overrideConfig": {
      "sessionId": "AMBER_FLUENT_SESSION_ID_01",
      "anthropicApiKey": "AMBER_FLUENT_ANTHROPIC_KEY_01",
      "stripNewLines": true,
      "modelName": {
        "chatAnthropic_0": "claude-3-sonnet-20240229",
        "chatOpenAI_0": "gpt-3.5-turbo-16k",
        "openAIEmbeddings_0": "text-embedding-3-small"
      },
      "openAIApiKey": {
        "openAIEmbeddings_0": "AMBER_FLUENT_OPENAI_API_KEY_01",
        "chatOpenAI_0": "AMBER_FLUENT_OPENAI_API_KEY_01"
      },

      "serpApiKey": "AMBER_FLUENT_SERPAPI_KEY_01",
      "SystemMessage": "You are a helpful assistant. Help the user answer any questions.\n\nYou have access to the following tools:\n\n{tools}\n\nIn order to use a tool, you can use <tool></tool> and <tool_input></tool_input> tags. You will then get back a response in the form <observation></observation>\nFor example, if you have a tool called 'search' that could run a google search, in order to search for the weather in SF you would respond:\n\n<tool>search</tool><tool_input>weather in SF</tool_input>\n<observation>64 degrees</observation>\n\nWhen you are done, respond with a final answer between <final_answer></final_answer>. For example:\n\n<final_answer>The weather in SF is 64 degrees</final_answer>\n\nBegin!\n\nPrevious Conversation:\n{chat_history}\n\nQuestion: {input}\n{agent_scratchpad}",
      "temperature": 0.2

    },
    "timeout_ms": 50000
  }
```

Notice how there are entries like, `AMBER_FLUENT_OPENAI_API_KEY_01` that match the entries we created in the amber vault.   

At runtime, fluent will look up any variable that begins with `AMBER_` and export it's decrypted secret as an environmental variable that is cleaned up at the end of execution when fluent returns the value.  

This works for any data stored in the amber vault.  Just make certain that the value in the config.json is the same value as the key you want in the amber vault.

The chatflow example in the Flowise install is basically blank as far as configuration.  Everything is picked up at runtimne during the submission.  

So your queries will go to Anthropic with your keys.  Queries are logged through the flowise install, so be aware of what you send this this public server.  Otherwise, anything you send from fluent will be with your keys and your data to the service.  

![CleanShot 2024-04-22 at 21 28 03](https://github.com/njfio/fluent_cli/assets/7220/bb94fe7f-6a3a-4d77-a644-4e12e4ba3879)


The configuration options support all the overrides presented by Flowise and are also useful in calling generic webhooks.

The minimum configuration for a flow in config.json is below.  
```json
{
    "name": "SonnetXMLAgentAnowtherWebService",
    "protocol": "https",
    "hostname": "container-qygwpcc.containers.anotherwebservice.com",
    "port": 3000,
    "chat_id": "8ea46fa9-4aef-4184-a399-c588c576d148",
    "request_path": "/api/v1/prediction/",
    "sessionId": "",
    "bearer_token": "AMBER_ANOTHERWEBSERVICE_NJF",
    "overrideConfig": {
    }
}   
```


It is also possible to use fluent to invoke generic webhooks through services like Make and Zapier.
```json
  {
    "name": "MakeLeonardoImagePost",
    "protocol": "https",
    "hostname": "hook.us1.make.com",
    "port": 443,
    "chat_id": "19riyltebstlvc3q1tvei7s7jduld8xa",
    "request_path": "/",
    "sessionId": "",
    "bearer_token": "AMBER_MAKE_LEONARDO_IMAGE_POST",
    "overrideConfig": {
      "modelID": "AMBER_LEONARDO_AI_KINO_XL_MODEL_ID",
      "negative_prompt": "words, letters, symbols, hands, deformities, low-quality,",
      "alchemy": true,
      "photoReal": true,
      "photoRealVersion":"v2",
      "presetStyle": "CINEMATIC",
      "makeAuthentication": "AMBER_MAKE_LEONARDO_IMAGE_POST",
      "seed": ""
    },
    "timeout_ms": 5000000
  }
```


This works because the payload and request builders work off this information, the make webhook sees the various fields and is configured to return the images in markdown format.  

There are plenty of working chatflows publically available that will function correctly once your api keys are in amber, I will add more and include the chatflow exports or make blueprints as well.

## Final Check

Before running fluent for the first time, ensure you have keys configured in your yaml store.  

Also make sure that the follwoing USER environmental variables are set.


Example of setting an environment variable:
```bash
export FLUENT_CLI_CONFIG_PATH="/path/to/your/config.json"
export AMBER_YAML='/path/to/amber.yaml'
```


Also, you need to source the autocomplete files for your shell environment. These files are located in the source code.  There is a generate function for bash but it will just generate the file as it's store in the code repository.

```powershell
notepad $profile
#add to the file
'. ~/.fluent_cli_autocomplete.ps1' // or wherever your file is located
```

```bash
open ~/.zshrc
#add to the file
source /Users/n/.fluent_cli/fluent_cli_autocomplete.sh   // Make sure it's executive 'chmod +x /Users/n/.fluent_cli/fluent_cli_autocomplete.sh'
```

## Usage

To interact with FlowiseAI workflows, use the following syntax:

```bash
fluent [options] <command> [arguments]
```

##s Examples:

#### At it's simplest.

```bash
fluent SonnetXMLAgentAnowtherWebService "Tell me about the beautiful sky"
```

#### Taking stdin as context.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: "
```

#### Taking stdin as context and adding an inline system prompt.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: " -i 'you can only speak in spanish'
```

#### Taking stdin as context and adding an system prompt through a file.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: " -f src/system_prompts/german_prompt.txt
```

#### Taking stdin as context and adding an system prompt through a file and adding an additional input file to the context.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: " -f src/system_prompts/german_prompt.txt -a ~/Downloads/AIsGreatestHits.xls
```

#### Taking stdin as context and adding an system prompt through a file and adding an additional input file to the context and piping the output of that command into another fluent.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: " -f src/system_prompts/german_prompt.txt -a ~/Downloads/AIsGreatestHits.xls \
fluent GroqMixtral8x7bAgentAnotherWebService 'Summarize the details and provide new insight"
```

#### Taking stdin as context and adding an system prompt through a file and adding an additional input file to the context and piping the output of that command into another fluent and asking the second flow to provide a command.  The -p output modifier looks for the content inside markdown code block syntax ``` content ``` and returns it only, and that response is directed to an output file.

```bash
cat ~/ExampleFolder/ExampleFile | fluent SonnetXMLAgentAnowtherWebService "Tell me about this context: " -f src/system_prompts/german_prompt.txt -a ~/Downloads/AIsGreatestHits.xls \
fluent GroqMixtral8x7bAgentAnotherWebService 'Summarize the details and provide new insight and provide a python script to better understand the details" -p > ~/my/output/file/path
```

#### Taking stdin as context and adding an system prompt through a file and adding an additional input file to the context and uploading a file to a workflow.

```bash
cat ~/ExampleFolder/ExampleFile | fluent GPT4ImageUpload "Tell me about this context and describe the image" -f src/system_prompts/german_prompt.txt -a ~/Downloads/AIsGreatestHits.xls -u ~/Downloads/myupload.png
```

#### Taking stdin as context and sending it to a Make workflow to get an image generated from Leonardo.ai and downloads the result to ~/Downloads.

```bash
cat ~/ExampleFolder/ExampleFile | fluent MakeLeonardoImagePost "Imagine a beautiful magical dragon kite" -d ~/Downloads
```


### Commands

```bash
Interacts with FlowiseAI workflows

USAGE:
    fluent [OPTIONS] [ARGS]

ARGS:
    <flowname>    The flow name to invoke
    <request>     The request string to send

OPTIONS:
    -a, --additional-context-file <additional-context-file>
            Specifies a file from which additional request context is loaded

    -c <context>
            Optional context to include with the request

    -d, --download-media <DIRECTORY>
            Downloads all media files listed in the output to a specified directory

    -f, --system-prompt-override-file <system-prompt-override-file>
            Overrides the system message from a specified file

    -g, --generate-autocomplete
            Generates a bash autocomplete script

    -h, --help
            Print help information

    -i, --system-prompt-override-inline <system-prompt-override-inline>
            Overrides the system message with an inline string

    -m, --markdown-output
            Outputs the response to the terminal in stylized markdown. Do not use for pipelines

    -p, --parse-code-output
            Extracts and displays only the code blocks from the response

    -u, --upload-image-path <FILE>
            Sets the input file to use

    -V, --version
            Print version information

    -z, --full-output
            Outputs all response data in JSON format
```

## Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

This is roughly the code flow.

![CleanShot 2024-04-21 at 22 59 12](https://github.com/njfio/fluent_cli/assets/7220/e9d0023b-5f63-4a22-ae26-e948d3ec262f)


## License

Distributed under the MIT License. See `LICENSE` for more information.

## Contact

Nicholas Ferguson - nick@njf.io

Project Link: [https://github.com/njfio/fluent-cli](https://github.com/njfio/fluent-cli)


