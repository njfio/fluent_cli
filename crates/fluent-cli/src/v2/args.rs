use clap::{ArgAction, Args, Parser, Subcommand, ValueHint};
use fluent_sdk::ai::openai::FluentOpenAIChatRequest;

#[derive(Parser, Debug)]
#[command(name = "fluent-cli")]
pub struct FluentArgs {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Execute a pipeline")]
    Pipeline(Pipeline),
    #[command(about = "Execute an openai request")]
    OpenAiChat {
        #[command(flatten)]
        shared: RequestSharedArgs,
        #[command(flatten)]
        request: FluentOpenAIChatRequest,
    },
}

#[derive(Debug, Args)]
pub struct RequestSharedArgs {
    #[arg(short = 'a', long = "additional-context-file", value_hint = ValueHint::FilePath, help = "Specifies a file from which additional request context is loaded")]
    pub additional_context_file: Option<String>,

    #[arg(long, conflicts_with = "request", action = ArgAction::SetTrue, help = "Enables upsert mode")]
    pub upsert: bool,

    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Specifies the input file or directory to process"
    )]
    pub input: Option<String>,

    #[arg(
        short = 't',
        long,
        value_name = "TERMS",
        help = "Comma-separated list of metadata terms"
    )]
    pub metadata: Option<String>,

    #[arg(
        short = 'l',
        long = "upload_image_file",
        value_name = "FILE",
        help = "Upload a media file"
    )]
    pub upload_image_file: Option<String>,

    #[arg(
        short = 'd',
        long = "download-media",
        value_name = "DIR",
        help = "Download media files from the output"
    )]
    pub download_media: Option<String>,

    #[arg(short = 'p', long = "parse-code", action = ArgAction::SetTrue, help = "Parse and display code blocks from the output")]
    pub parse_code: bool,

    #[arg(short = 'x', long = "execute-output", action = ArgAction::SetTrue, help = "Execute code blocks from the output")]
    pub execute_output: bool,

    #[arg(short = 'm', long = "markdown", action = ArgAction::SetTrue, help = "Format output as markdown")]
    pub markdown: bool,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Generate and execute a Cypher query based on the given string"
    )]
    pub generate_cypher: Option<String>,
}

#[derive(Debug, Args)]
pub struct Pipeline {
    #[arg(short, long, help = "The YAML file containing the pipeline definition")]
    pub file: String,

    #[arg(short, long, help = "The input for the pipeline")]
    pub input: String,

    #[arg(long, help = "Force a fresh execution of the pipeline")]
    pub force_fresh: bool,

    #[arg(long, help = "Specify a run ID for the pipeline")]
    pub run_id: Option<String>,

    #[arg(
        long,
        help = "Output only the JSON result, suppressing PrintOutput steps"
    )]
    pub json_output: bool,
}
