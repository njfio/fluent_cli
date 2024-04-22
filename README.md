
## Fluent CLI: Streamline Your Workflows with Precision and Ease

**Fluent CLI** is an advanced command-line interface designed to interact seamlessly with multiple workflow systems like FlowiseAI, Make, and Zapier. Tailored for developers, IT professionals, and power-users, Fluent CLI facilitates robust automation, simplifies complex interactions, and enhances productivity through a powerful and configurable command suite.

### Key Features:

- **Multi-Service Integration**: Connect effortlessly with services like FlowiseAI, Make, and Zapier to automate and manage workflows across different platforms.

- **Dynamic Configuration**: Utilize JSON-based configurations to dynamically adjust command parameters, making your workflows flexible and adaptable to changing needs.

- **Enhanced File Handling**: Support for uploading images and files directly through the CLI, integrated smoothly with asynchronous operations to boost performance.

- **Secure Environment Interaction**: Automatic handling of environment variables and secure token management ensures that your operations are safe and your data is protected.

- **Versatile Output Options**: Whether you need beautifully formatted markdown, concise code blocks, or well-structured JSON, Fluent CLI delivers your data in the format you prefer, right in your terminal.- **Interactive Inputs**: Fluent CLI handles stdin inputs gracefully, allowing for interactive user sessions and seamless piping from other commands.

- **Autocomplete Workflow Names**: Fluent CLI includes autocomplete for all the configured workflow names which makes calling any of the workflows just a few keystrokes away.

- **Versionable Secure Vault**: Fluent CLI is integrated with [amber](https://github.com/fpco/amber), as a secure vault for configuration information.


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

   ## or install directly

   cargo install --git https://github.com/njfio/fluent_cli.git
   ```
2. Navigate into the project directory:
   ```bash
   cd fluent-cli
   ```
3. Build the project using Cargo (Rust's package manager):
   ```bash
   cargo build --release
   ```

## Configuration

Fluent CLI relies on a JSON configuration file to manage the workflow specifics. Ensure that the `FLUENT_CLI_CONFIG_PATH` environment variable is set to point to your configuration file.

Example of setting an environment variable:
```bash
export FLUENT_CLI_CONFIG_PATH="/path/to/your/config.json"
```

## Usage

To interact with FlowiseAI workflows, use the following syntax:

```bash
fluent [options] <command> [arguments]
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


