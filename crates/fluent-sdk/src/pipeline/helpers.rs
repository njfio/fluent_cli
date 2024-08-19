use std::{env, sync::Arc};

use super::model::{TransferData, TransferDataValue};
use crate::ai::{FluentRequest, FluentResponse, FluentSdkRequest};
use anyhow::{anyhow, Context, Result};

async fn execute_llm_request(
    input: Option<&Arc<TransferData>>,
    request: FluentRequest,
    append_history: bool,
) -> Result<(FluentRequest, FluentResponse)> {
    let mut request = request.as_request();
    let mut input_as_string = if let Some(input) = input {
        if append_history {
            input.history_as_string()
        } else {
            input.string_value()
        }
    } else {
        "".to_string()
    };
    if let Some(prompt) = request.request {
        input_as_string = format!("{}\nINPUT DATA:\n{}", prompt, input_as_string);
    };
    request.request = Some(input_as_string);
    let response = request.run().await?;
    Ok((request, response))
}

pub async fn process_llm_request(
    input: Option<&Arc<TransferData>>,
    request: FluentRequest,
    append_history: bool,
) -> Result<TransferDataValue> {
    let (request, response) = execute_llm_request(input, request, append_history).await?;
    Ok(TransferDataValue::Fluent(request, response))
}

pub async fn execute_evaluation(
    input: Option<&Arc<TransferData>>,
    mut request: FluentRequest,
    append_history: bool,
) -> Result<bool> {
    if let Some(request) = request.request.as_mut() {
        request.push_str("\nThe answer should be a lowercase true or false. Do not include any other information in your response.");
    }
    execute_llm_request(input, request, append_history)
        .await
        .and_then(|(request, response)| {
            /*
            tracing::info!(
                "Evaluating: {:?}: {}",
                request.request,
                response.data.content
            );
            */

            response
                .data
                .content
                .parse()
                .inspect_err(|_| tracing::error!("Response data was [{}]", response.data.content))
                .context("Failed to parse response as boolean")
        })
}

pub fn resolve_env_var(value: &mut serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::String(v) if v.starts_with("ENV_") => {
            let env_key = &v[4..]; // Skip the "ENV_" prefix to fetch the correct env var
            match env::var(env_key) {
                Ok(env_value) => *v = env_value,
                Err(e) => {
                    return Err(anyhow!(
                        "Failed to find environment variable '{}': {}",
                        env_key,
                        e
                    ))
                }
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            for (_, v) in map.iter_mut() {
                resolve_env_var(v)?;
            }
            Ok(())
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_env_var(item)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
