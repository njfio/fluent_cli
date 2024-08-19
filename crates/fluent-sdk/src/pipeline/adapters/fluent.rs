use std::{borrow::Borrow, sync::Arc};

use anyhow::{anyhow, Ok, Result};
use async_trait::async_trait;
use fluent_pipeline::{Evaluator, Executable};
use serde::{Deserialize, Serialize};

use crate::{
    ai::FluentSdkRequest,
    pipeline::helpers::{execute_evaluation, process_llm_request},
    prelude::{FluentOpenAIChatRequest, TransferData},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum FluentAdapterType {
    OpenAIChat(FluentOpenAIChatRequest),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct FluentAdapter {
    pub append_history: bool,
    pub r#type: FluentAdapterType,
}
#[async_trait]
impl Executable<Arc<TransferData>> for FluentAdapter {
    async fn execute(
        &self,
        input: Option<&Arc<TransferData>>,
    ) -> Result<Option<Arc<TransferData>>> {
        let value = match self.r#type.borrow() {
            FluentAdapterType::OpenAIChat(request) => {
                process_llm_request(input, request.as_request(), self.append_history).await?
            }
        };
        let data = Arc::new(TransferData {
            previous: input.cloned(),
            value,
        });
        tracing::info!("Fluent Response: {}", data.string_value());
        Ok(Some(data))
    }
}

#[async_trait]
impl Evaluator<Arc<TransferData>> for FluentAdapter {
    async fn evaluate(&self, input: Option<&Arc<TransferData>>) -> Result<bool> {
        if input.is_none() {
            return Err(anyhow!("No input data"));
        }
        let value = match self.r#type.borrow() {
            FluentAdapterType::OpenAIChat(request) => {
                execute_evaluation(input, request.as_request(), self.append_history).await?
            }
        };
        Ok(value)
    }
}
