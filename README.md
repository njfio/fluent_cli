

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
fluent-cli [options] <command> [arguments]
```

### Commands

- `--generate-autocomplete`: Generates a bash script for command-line autocompletion.
- `--system-prompt-override-inline`: Overrides the system prompt with an inline string.
- `--upload-image-path`: Specifies the path to an image file to be uploaded within the request.

For detailed usage of each command, refer to the help output:

```bash
fluent-cli --help
```

## Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## License

Distributed under the MIT License. See `LICENSE` for more information.

## Contact

Your Name - your.email@example.com

Project Link: [https://github.com/your-username/fluent-cli](https://github.com/your-username/fluent-cli)

---

This README template should give users and contributors a clear understanding of what Fluent CLI is, how it can be used, and how to get involved with its development. Adjust the contact details, repository URLs, and specific command examples to match the actual details of your project.
