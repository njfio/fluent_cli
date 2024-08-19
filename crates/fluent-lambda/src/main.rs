use fluent_sdk::prelude::*;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub data: fluent_core::types::Response,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    lambda_runtime::run(service_fn(|event: LambdaEvent<FluentRequest>| async {
        lambda_handler(event).await
    }))
    .await
}

#[tracing::instrument(skip(event), fields(req_id = %event.context.request_id))]
async fn lambda_handler(event: LambdaEvent<FluentRequest>) -> Result<Response, Error> {
    event
        .payload
        .run()
        .await
        .map_err(Error::from)
        .map(|r| Response { data: r.data })
}
