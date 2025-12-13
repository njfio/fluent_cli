use fluent_sdk::FluentRequest;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

// Cold start tracking
static COLD_START: AtomicBool = AtomicBool::new(true);
static START_TIME: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(Instant::now);

// Input size limit: 1MB
const MAX_INPUT_SIZE: usize = 1024 * 1024;

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub data: fluent_core::types::Response,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
    error_type: String,
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

#[derive(Debug, Serialize)]
struct PayloadTooLargeResponse {
    error: String,
    max_size_bytes: usize,
    actual_size_bytes: usize,
    request_id: String,
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
    let request_id = event.context.request_id.clone();

    // Log cold start information
    let is_cold_start = COLD_START.swap(false, Ordering::SeqCst);
    if is_cold_start {
        let init_duration = START_TIME.elapsed();
        tracing::info!(
            cold_start = true,
            init_duration_ms = init_duration.as_millis() as u64,
            "Lambda cold start"
        );
    }

    // Check input size
    let payload_size = match serde_json::to_string(&event.payload) {
        Ok(s) => s.len(),
        Err(e) => {
            tracing::error!(error = %e, "Failed to serialize payload for size check");
            return Err(format!("Failed to serialize payload: {}", e).into());
        }
    };

    if payload_size > MAX_INPUT_SIZE {
        tracing::warn!(payload_size, max_size = MAX_INPUT_SIZE, "Payload too large");

        let error_body = PayloadTooLargeResponse {
            error: "Payload too large".to_string(),
            max_size_bytes: MAX_INPUT_SIZE,
            actual_size_bytes: payload_size,
            request_id,
        };

        return Err(serde_json::to_string(&error_body)
            .unwrap_or_else(|_| "Payload too large".to_string())
            .into());
    }

    // Process request
    match event.payload.run().await {
        Ok(r) => Ok(Response { data: r.data }),
        Err(e) => {
            tracing::error!(error = %e, "Request processing failed");
            Err(format_error(&e, &request_id).into())
        }
    }
}

fn format_error(err: &anyhow::Error, request_id: &str) -> String {
    let error_response = ErrorResponse {
        error: err.to_string(),
        error_type: classify_error(err),
        request_id: request_id.to_string(),
        details: if cfg!(debug_assertions) {
            Some(format!("{:?}", err))
        } else {
            None
        },
    };

    serde_json::to_string(&error_response)
        .unwrap_or_else(|_| format!(r#"{{"error":"{}","request_id":"{}"}}"#, err, request_id))
}

fn classify_error(err: &anyhow::Error) -> String {
    let msg = err.to_string().to_lowercase();
    if msg.contains("config") {
        "ConfigError"
    } else if msg.contains("auth") || msg.contains("api key") || msg.contains("unauthorized") {
        "AuthError"
    } else if msg.contains("timeout") {
        "TimeoutError"
    } else if msg.contains("not found") {
        "NotFoundError"
    } else if msg.contains("invalid") || msg.contains("parse") {
        "ValidationError"
    } else if msg.contains("network") || msg.contains("connection") {
        "NetworkError"
    } else {
        "InternalError"
    }
    .to_string()
}
