use fluent_sdk::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let request = FluentOpenAIChatRequest::builder()
        .prompt("Hello, world!".to_string())
        .openai_key("YOUR_OPENAI_KEY".to_string())
        .build()?;

    let response = request.run().await?;
    println!("{:?}", response.data);
    Ok(())
}
