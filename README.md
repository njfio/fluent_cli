

# Fluent CLI

Fluent CLI is a powerful command-line interface designed to interact seamlessly with the FlowiseAI workflows. It provides a flexible and efficient way to handle complex workflow interactions and automation tasks directly from the command line.   


## Features

- **Dynamic Configuration**: Fluent CLI utilizes a JSON-based configuration system that enables dynamic adjustments of workflow parameters.
- **Advanced Logging**: Detailed debugging and logging capabilities to trace and optimize interactions.
- **File Handling**: Support for uploading and downloading images and handling files asynchronously within the workflow interactions.
- **Environment Variable Integration**: Automatic decryption and integration of environment variables for secure operations and transportable configurations.
- **Extensible**: Easy to extend with additional commands and configurations.

## Installation

To set up Fluent CLI on your local system, follow these steps:

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/fluent-cli.git
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

- `--generate-autocomplete`: Generates a bash script for command-line autocompletion.
- `--system-prompt-override-inline`: Overrides the system prompt with an inline string.
- `--upload-image-path`: Specifies the path to an image file to be uploaded within the request.

For detailed usage of each command, refer to the help output:

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

Your Name - nick@njf.io

Project Link: [https://github.com/njfio/fluent-cli](https://github.com/njfio/fluent-cli)


