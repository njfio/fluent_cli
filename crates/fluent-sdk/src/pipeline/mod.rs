pub mod adapters;
pub mod helpers;
pub mod model;

#[cfg(test)]
mod tests {
    use std::{env, sync::Arc};

    use crate::prelude::*;
    use anyhow::Result;
    use fluent::{FluentAdapter, FluentAdapterType};

    #[tokio::test]
    async fn test_parse_pipeline_config() -> Result<()> {
        //let config = PipelineConfig::try_from(include_str!("bob_alice.yaml"))?;
        let _ = tracing_subscriber::fmt::try_init();
        let openai_bearer_token = env::var("OPENAI_KEY")?;
        let openai_bearer_token = openai_bearer_token.as_str();
        let config = PipelineConfig::default()
            .node(
                NodeConfig::default()
                    .name("Start")
                    .r#type(NodeType::Start)
                    .next("Loop")
            )
            .node(
                NodeConfig::default()
                    .name("Loop")
                    .r#type(NodeType::Join)
                    .next("Bob"),
            )
            .node(
                NodeConfig::default()
                    .name("Bob")
                    .r#type(NodeType::Task(Task::Fluent(FluentAdapter{
                        append_history: true,
                        r#type: FluentAdapterType::OpenAIChat(
                            FluentOpenAIChatRequestBuilder::default()
                                .bearer_token(openai_bearer_token)
                                .prompt(r#" 
                                    Instruction: you are Bob.
                                    You are a human.
                                    You are chatting with Alice.
                                    You are trying to get to know her better.
                                    You are trying to be friendly and polite.
                                    You are trying to be respectful.
                                    Every time you send a message, put Bob: at the beginning of the message.
                                "#,
                                )
                                .build()?,
                        )
                    })))
                    .next("Alice"),
            )
            .node(
                NodeConfig::default()
                    .name("Alice")
                    .r#type(NodeType::Task(Task::Fluent(FluentAdapter{
                        append_history: true,
                        r#type: FluentAdapterType::OpenAIChat(
                            FluentOpenAIChatRequestBuilder::default()
                                .bearer_token(openai_bearer_token)
                                .prompt(r#" 
                                    Instruction: you are Alice.
                                    You are a human.
                                    You are chatting with Bob.
                                    You are trying to get to know him better.
                                    You are trying to be friendly and polite.
                                    You are trying to be respectful.
                                    Every time you send a message, put Alice: at the beginning of the message.
                                "#,
                                )
                                .build()?,
                        )
                    })))
                    .next("Decision"),
            )
            .node( NodeConfig::default()
                    .name("Decision")
                    .r#type(NodeType::Decision)
                    .next_if("Loop", 
                        Condition::Fluent(FluentAdapter{
                        append_history: true,
                        r#type: FluentAdapterType::OpenAIChat(
                            FluentOpenAIChatRequestBuilder::default()
                                .bearer_token(openai_bearer_token)
                                .prompt(r#" 
                                    You will receive interactions between Bob and Alice as Input data.
                                    Only consider the last input data you have received
                                    Alice messages are prefixed with Alice: and Bob messages are prefixed with Bob:.
                                    You need to count the number of messages from Alice and answer the following question:
                                    The number of Alice messages is less than 5.
                                "#,
                                )
                                .build()?
                            )
                        }), true)
                    .next("End"),
            )
            .node(
                NodeConfig::default()
                    .name("End")
                    .r#type(NodeType::End),
            )
        ;
        assert!(!config.nodes.is_empty(), "The pipeline should have nodes.");
        let pipeline: Pipeline<Arc<TransferData>> = config.try_into()?;
        pipeline.run(None).await?;
        Ok(())
    }
}
