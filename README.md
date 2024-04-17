# fluent_cli
home of the fluent_cli_1.0

Installation Directory
```sh
~/.fluent_cli
```

Files in Install Path
```sh
~/.fluent_cli/fluent_system_config.json
~/.fluent_cli/fluent_flow_config.json
```

Create the ENV variables
```sh
export FLUENT_CLI_SYS_CONFIG_PATH=/path/to/your/user/.fluent_cli/fluent_system_config.json
export FLUENT_CLI_FLOW_CONFIG_PATH/path/to/your/user/.fluent_cli/fluent_flow_config.json
```


```/fluent_cli
|-- /src
    |-- main.rs         # Main source file
|-- Cargo.toml          # Dependency and project configuration
|-- .gitignore          # Git ignore file
```

# Orginization
```
/fluent_cli//fluent_cli
|-- /src
    |-- main.rs        # Entry point, handles CLI interaction
    |-- lib.rs         # Core functionality and shared structures
    |-- /config        # Configuration management
        |-- mod.rs     # Configuration module entry
        |-- loader.rs  # Configuration loading functionality
    |-- /api           # API interaction
        |-- mod.rs
        |-- client.rs  # API client functionalities
|-- Cargo.toml

```