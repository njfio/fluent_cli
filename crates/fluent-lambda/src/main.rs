use fluent_lambda::{run, LambdaRequest, Response};
use lambda_runtime::{service_fn, Error, LambdaEvent};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    lambda_runtime::run(service_fn(|event: LambdaEvent<LambdaRequest>| async {
        lambda_handler(event).await
    }))
    .await
}

#[tracing::instrument(skip(event), fields(req_id = %event.context.request_id))]
async fn lambda_handler(event: LambdaEvent<LambdaRequest>) -> Result<Response, Error> {
    run(event.payload).await.map_err(Error::from)
}
