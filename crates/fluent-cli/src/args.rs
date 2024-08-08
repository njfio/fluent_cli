use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "my_app")]
struct FluentArgs {
    #[arg(short, long, value_name = "FILE")]
    config: Option<String>,

    #[arg(help = "The engine to use (openai or anthropic)")]
    engine: String,

    #[arg(help = "The request to process")]
    request: Option<String>,

    #[arg(short, long, value_name = "KEY=VALUE", action = ArgAction::Append, num_args(1..))]
    override_config: Vec<String>,

    #[arg(long, short, help = "Specifies a file from which additional request context is loaded", value_hint = clap::ValueHint::FilePath)]
    additional_context_file: Option<String>,

    #[arg(long, help = "Enables upsert mode", conflicts_with = "request")]
    upsert: bool,

    #[arg(
        long,
        short,
        value_name = "FILE",
        help = "Input file or directory to process (required for upsert)"
    )]
    input: Option<String>,

    #[arg(
        long,
        short,
        value_name = "TERMS",
        help = "Comma-separated list of metadata terms (for upsert)"
    )]
    metadata: Option<String>,

    #[arg(short, long, value_name = "FILE", help = "Upload a media file")]
    upload_image_file: Option<String>,

    #[arg(
        short,
        long,
        value_name = "DIR",
        help = "Download media files from the output"
    )]
    download_media: Option<String>,

    #[arg(short, long, help = "Parse and display code blocks from the output")]
    parse_code: bool,

    #[arg(short, long, help = "Execute code blocks from the output")]
    execute_output: bool,

    #[arg(short, long, help = "Format output as markdown")]
    markdown: bool,

    #[arg(
        long,
        value_name = "QUERY",
        help = "Generate and execute a Cypher query based on the given string"
    )]
    generate_cypher: Option<String>,

    #[command(subcommand)]
    pipeline: Option<Pipeline>,
}

#[derive(Subcommand, Debug)]
enum Pipeline {
    #[command(about = "Execute a pipeline")]
    Execute {
        #[arg(short, long, help = "The YAML file containing the pipeline definition")]
        file: String,

        #[arg(short, long, help = "The input for the pipeline")]
        input: String,

        #[arg(long, help = "Force a fresh execution of the pipeline")]
        force_fresh: bool,

        #[arg(long, help = "Specify a run ID for the pipeline")]
        run_id: Option<String>,

        #[arg(
            long,
            help = "Output only the JSON result, suppressing PrintOutput steps"
        )]
        json_output: bool,
    },
}
