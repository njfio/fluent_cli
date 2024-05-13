const completion: Fig.Spec = {
  name: "fluent",
  description: "Interacts with FlowiseAI, Langflow, and Webhook workflows",
  args: [
    {
      name: "flowname",
      isCommand:true,
      generators: [{
        script: ["jq",  "-r", ".[].name", "/Users/n/RustroverProjects/fluent_cli/fluent_cli/config.json"],
      postProcess: (out: string) => {
        return out.split('\n').filter(Boolean).map((name) => {  
          return { name: name, description: "flow name"}
        })
      }
      }]
    },
    {
      name: "request",
      description: "Specify the request",
      isOptional: false,
      isCommand: false,
    },
  ],
  options: [
    {
      name: "-c",
      description: "Optional context to include with the request",
      isRepeatable: false,
      args: {
        name: "context",
        isOptional: true,
      },
    },
    {
      name: ["-i", "--system-prompt-override-inline"],
      description: "Overrides the system message with an inline string",
      isRepeatable: false,
      args: {
        name: "system-prompt-override-inline",
        isOptional: true,
      },
    },
    {
      name: ["-f", "--system-prompt-override-file"],
      description: "Overrides the system message from a specified file",
      isRepeatable: false,
      args: {
        name: "system-prompt-override-file",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
      },
    },
    {
      name: ["-a", "--additional-context-file"],
      description: "Specifies a file from which additional request context is loaded",
      isRepeatable: false,
      args: {
        name: "additional-context-file",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
      },
    },
    {
      name: ["-u", "--upload-image-path"],
      description: "Sets the input file to use",
      isRepeatable: false,
      args: {
        name: "upload-image-path",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
      },
    },
    {
      name: ["-d", "--download-media"],
      description: "Downloads all media files listed in the output to a specified directory",
      isRepeatable: false,
      args: {
        name: "download-media",
        template: "folders",
        isVariadic: true,
        isOptional: true,
      },
    },
    {
      name: "--upsert-no-upload",
      description: "Sends a JSON payload to the specified endpoint without uploading files",
      isRepeatable: false,
      args: {
        name: "upsert-no-upload",
        isOptional: true,
      },
    },
    {
      name: "--upsert-with-upload",
      description: "Uploads a file to the specified endpoint",
      isRepeatable: false,
      args: {
        name: "upsert-with-upload",
        template: "filepaths",
        isVariadic: true,
        isOptional: true,
      },
    },
    {
      name: ["--generate-autocomplete"],
      description: "Generates a bash autocomplete script",

    },
    {
      name: "--generate-fig-autocomplete",
      description: "Generates a fig autocomplete script",
    },
    {
      name: ["-p", "--parse-code-output"],
      description: "Extracts and displays only the code blocks from the response",
    },
    {
      name: ["-z", "--full-output"],
      description: "Outputs all response data in JSON format",
    },
    {
      name: ["-m", "--markdown-output"],
      description: "Outputs the response to the terminal in stylized markdown. Do not use for pipelines",
    },
    {
      name: "--webhook",
      description: "Sends the command payload to the webhook URL specified in config.json",
    },
    {
      name: ["-h", "--help"],
      description: "Print help",
    },
    {
      name: ["-V", "--version"],
      description: "Print version",
    },
  ],
};

export default completion;
