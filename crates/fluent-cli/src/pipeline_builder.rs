use anyhow::Result;
use dialoguer::{Confirm, Input, Select};
use fluent_engines::pipeline_executor::{FileStateStore, Pipeline, PipelineExecutor, PipelineStep};
use std::io::stdout;
use std::path::PathBuf;
use termimad::crossterm::{
    execute,
    terminal::{Clear, ClearType},
};

pub async fn build_interactively() -> Result<()> {
    execute!(stdout(), Clear(ClearType::All))?;
    let name: String = Input::new().with_prompt("Pipeline name").interact_text()?;

    let mut pipeline = Pipeline {
        name,
        steps: Vec::new(),
    };

    loop {
        execute!(stdout(), Clear(ClearType::All))?;
        println!("Building pipeline: {}", pipeline.name);
        for (idx, step) in pipeline.steps.iter().enumerate() {
            println!("{}. {:?}", idx + 1, step_name(step));
        }
        let action = Select::new()
            .with_prompt("Choose action")
            .items(&["Add step", "Finish"])
            .default(0)
            .interact()?;
        if action == 1 {
            break;
        }

        let step_type = Select::new()
            .with_prompt("Step type")
            .items(&["Command", "ShellCommand", "PrintOutput"])
            .default(0)
            .interact()?;

        match step_type {
            0 => {
                let name: String = Input::new().with_prompt("Step name").interact_text()?;
                let command: String = Input::new().with_prompt("Command").interact_text()?;
                let save: String = Input::new()
                    .with_prompt("Save output variable (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                pipeline.steps.push(PipelineStep::Command {
                    name,
                    command,
                    save_output: if save.is_empty() { None } else { Some(save) },
                    retry: None,
                });
            }
            1 => {
                let name: String = Input::new().with_prompt("Step name").interact_text()?;
                let command: String = Input::new().with_prompt("Shell command").interact_text()?;
                let save: String = Input::new()
                    .with_prompt("Save output variable (optional)")
                    .allow_empty(true)
                    .interact_text()?;
                pipeline.steps.push(PipelineStep::ShellCommand {
                    name,
                    command,
                    save_output: if save.is_empty() { None } else { Some(save) },
                    retry: None,
                });
            }
            2 => {
                let name: String = Input::new().with_prompt("Step name").interact_text()?;
                let value: String = Input::new().with_prompt("Value").interact_text()?;
                pipeline
                    .steps
                    .push(PipelineStep::PrintOutput { name, value });
            }
            _ => {}
        }
    }

    let file: String = Input::new()
        .with_prompt("Save pipeline to file")
        .interact_text()?;
    let yaml = serde_yaml::to_string(&pipeline)?;
    std::fs::write(&file, yaml)?;
    println!("Pipeline saved to {}", file);

    if Confirm::new().with_prompt("Run pipeline now?").interact()? {
        let input: String = Input::new().with_prompt("Pipeline input").interact_text()?;
        let state_store_dir = PathBuf::from("./pipeline_states");
        tokio::fs::create_dir_all(&state_store_dir).await?;
        let state_store = FileStateStore {
            directory: state_store_dir,
        };
        let executor = PipelineExecutor::new(state_store, false);
        executor.execute(&pipeline, &input, false, None).await?;
    }
    Ok(())
}

fn step_name(step: &PipelineStep) -> &str {
    match step {
        PipelineStep::Command { name, .. } => name,
        PipelineStep::ShellCommand { name, .. } => name,
        PipelineStep::PrintOutput { name, .. } => name,
        _ => "step",
    }
}
