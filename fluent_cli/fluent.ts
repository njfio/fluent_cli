const completion: Fig.Spec = {
  name: "fluent",
  description: "Interacts with FlowiseAI, Langflow, and Webhook workflows",
  cache: false,
  priority: 100000,
  parserDirectives: {
    optionsMustPrecedeArguments: false,
    flagsArePosixNoncompliant: true,
  },
  args: [
    {
      name: "flow",
      isScript: false,
      isOptional: false,
      generators: {
        script: ["jq", "-r", '.[].name', "/Users/n/RustroverProjects/fluent_cli/fluent_cli/config.json"],
        postProcess: (out: string) => {
          return out.split('\n').filter(Boolean).map((name) => {
            return {
              name: name,
              description: "Flow name",
              icon: "🫠",
              insertValue: `${name} "{cursor}"`
            };
          });
        },
      },
    },
    {
      name: "request",
      optionsCanBreakVariadicArg: true,
      description: "Specify the request",
      isCommand: false,
      isOptional: false,
      isVariadic: true,
      default: "{cursor}", // Ensure cursor is positioned correctly inside quotes
    },
  ],
  options: [
    {
      name: ["-i", "--system-prompt-override-inline"],
      description: "Overrides the system message with an inline string",
      isRepeatable: false,
      priority: 99,
      args: {
        name: "system-prompt-override-inline",
        isOptional: true,
        description: "Inline system message",
      },
    },
    {
      name: ["-f", "--system-prompt-override-file"],
      description: "Overrides the system message from a specified file",
      isRepeatable: false,
      priority: 99,
      args: {
        name: "system-prompt-override-file",
        template: "filepaths",
        isVariadic: false,
        isOptional: true,
        description: "File containing system message",
      },
    },
    {
      name: ["-a", "--additional-context-file"],
      description: "Specifies a file from which additional request context is loaded",
      isRepeatable: false,
      priority: 500,
      args: {
        name: "additional-context-file",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
        description: "File containing additional context",
      },
    },
    {
      name: ["-u", "--upload-image-path"],
      description: "Sets the input file to use",
      priority: 150,
      isRepeatable: false,
      args: {
        name: "upload-image-path",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
        description: "File path for upload",
      },
    },
    {
      name: ["-d", "--download-media"],
      description: "Downloads all media files listed in the output to a specified directory",
      isRepeatable: false,
      priority: 500,
      args: {
        name: "download-media",
        template: "folders",
        isVariadic: true,
        isOptional: true,
        description: "Directory to download media files",
      },
    },
    {
      name: "--upsert-no-upload",
      description: "Sends a JSON payload to the specified endpoint without uploading files",
      isRepeatable: false,
      priority: 200,
      args: {
        name: "upsert-no-upload",
        isOptional: true,
        description: "JSON payload without upload",
      },
    },
    {
      name: "--upsert-with-upload",
      description: "Uploads a file to the specified endpoint",
      isRepeatable: false,
      priority: 200,
      args: {
        name: "upsert-with-upload",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
        description: "File path for upload",
      },
    },
    {
      name: "--generate-autocomplete",
      priority: 7,
      description: "Generates a bash autocomplete script",
      icon: "🔄",
    },
    {
      name: "--generate-fig-autocomplete",
      priority: 7,
      description: "Generates a fig autocomplete script",
      icon: "🔄",
    },
    {
      name: ["-p", "--parse-code-output"],
      priority: 400,
      description: "Extracts and displays only the code blocks from the response",
      exclusiveOn: ["-z", "--full-output", "-m", "--markdown-output"],
      icon: "💻",
    },
    {
      name: ["-z", "--full-output"],
      priority: 5,
      description: "Outputs all response data in JSON format",
      exclusiveOn: ["-p", "--parse-code-output", "-m", "--markdown-output"],
      icon: "📄",
    },
    {
      name: ["-m", "--markdown-output"],
      priority: 400,
      description: "Outputs the response to the terminal in stylized markdown. Do not use for pipelines",
      exclusiveOn: ["-z", "--full-output", "-p", "--parse-code-output"],
      icon: "📑",
    },
    {
      name: ["-h", "--help"],
      priority: 0,
      description: "Print help",
      icon: "❓",
    },
    {
      name: ["-V", "--version"],
      priority: 0,
      description: "Print version",
      icon: "ℹ️",
    },
    {
      name: ["-o", "--override"],
      description: "Overrides any entry in the config with the specified key-value pair",
      isRepeatable: true,
      priority: 150,
      args: {
        name: "override",
        description: "KEY=VALUE",
        isOptional: false,
        generators: {
          script: (tokens) => {
            const flowName = tokens[1]; // Assuming the flowname is the first argument after the command
            if (!flowName) return [];
            return [
              "jq",
              "-r",
              `map(select(.name == "${flowName}") | {overrideConfig: .overrideConfig, tweaks: .tweaks} | .[] | paths(scalars) as $p | ($p | map(tostring) | join("."))) | .[]`,
              "/Users/n/RustroverProjects/fluent_cli/fluent_cli/config.json"
            ];
          },
          postProcess: (out: string) => {
            return out.split('\n').filter(Boolean).map((key) => {
              return {
                name: key,
                description: "Config or tweak key",
                icon: "🔧",
              };
            });
          },
        },
      },
      icon: "🔧",
    },
  ],
};

export default completion;
