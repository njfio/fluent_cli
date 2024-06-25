use duckdb::{Connection, params, Result as DuckResult};
use crate::custom_error::{FluentError, FluentResult};
use std::sync::OnceLock;
use log::debug;

use serde::Deserialize;
use tokio::sync::Mutex;


static CONNECTION: OnceLock<Mutex<FluentResult<Connection>>> = OnceLock::new();

use crate::anthropic_agent_client::{AnthropicResponse, ContentBlock, Usage};
use crate::openai_agent_client::{OpenAIResponse, Choice, Message, ThreadData};
use crate::config::FlowConfig;
pub async fn get_connection() -> FluentResult<Connection> {
    let mutex = CONNECTION.get_or_init(|| {
        Mutex::new(
            Connection::open("fluent_cli.db")
                .map_err(|e| {
                    debug!("Error opening database connection: {:?}", e);
                    FluentError::from(e)
                })
                .and_then(|conn| {
                    debug!("Database connection opened successfully");
                    match create_tables(&conn) {
                        Ok(_) => Ok(conn),
                        Err(e) => {
                            debug!("Error in create_tables: {:?}", e);
                            Err(e)
                        }
                    }
                })
        )
    });

    let conn_result = mutex.lock().await;
    match conn_result.as_ref() {
        Ok(conn) => {
            debug!("Cloning existing database connection");
            conn.try_clone().map_err(|e| {
                debug!("Error cloning database connection: {:?}", e);
                FluentError::from(e)
            })
        },
        Err(e) => {
            debug!("Error getting database connection: {:?}", e);
            Err(e.clone())
        }
    }
}
fn create_tables(conn: &Connection) -> FluentResult<()> {
    debug!("Creating interactions table and sequence");
    conn.execute_batch(
        "CREATE SEQUENCE IF NOT EXISTS id_sequence START 1;

         CREATE TABLE IF NOT EXISTS interactions (
            id INTEGER PRIMARY KEY DEFAULT nextval('id_sequence'),
            timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            flow_name TEXT,
            flow_config TEXT,
            request TEXT,
            response TEXT,
            embedding FLOAT[],
            engine TEXT,
            duration FLOAT,
            -- Anthropic specific fields
            anthropic_id TEXT,
            anthropic_type TEXT,
            anthropic_role TEXT,
            anthropic_model TEXT,
            anthropic_stop_reason TEXT,
            anthropic_stop_sequence TEXT,
            anthropic_input_tokens INTEGER,
            anthropic_output_tokens INTEGER,
            -- OpenAI specific fields
            openai_id TEXT,
            openai_object TEXT,
            openai_created BIGINT,
            openai_model TEXT,
            openai_finish_reason TEXT,
            openai_prompt_tokens INTEGER,
            openai_completion_tokens INTEGER,
            openai_total_tokens INTEGER,
            -- OpenAI Assistant specific fields
            openai_assistant_id TEXT,
            openai_thread_id TEXT,
            openai_run_id TEXT,
            openai_message_id TEXT,
            openai_role TEXT,
            openai_content TEXT,
            openai_created_at BIGINT,
            openai_assistant_metadata TEXT,
            openai_thread_metadata TEXT,
            -- Google AI specific fields
            google_finish_reason TEXT,
            google_index INTEGER,
            google_safety_ratings TEXT
        );"
    ).map_err(|e| {
        debug!("Error creating interactions table and sequence: {:?}", e);
        FluentError::from(e)
    })?;
    debug!("Created interactions table and sequence successfully");
    Ok(())
}

pub fn log_interaction(
    conn: &Connection,
    flow_name: &str,
    request: &str,
    response: &str,
    engine: &str,
    duration: f64
) -> FluentResult<i64> {
    conn.execute(
        "INSERT INTO interactions (id, flow_name, request, response, engine, duration)
         VALUES (nextval('id_sequence'), ?, ?, ?, ?, ?)",
        params![flow_name, request, response, engine, duration]
    ).map_err(|e| {
        debug!("Error inserting interaction: {:?}", e);
        FluentError::from(e)
    })?;

    // Retrieve the last inserted ID
    let last_id: i64 = conn.query_row(
        "SELECT currval('id_sequence')",
        [],
        |row| row.get(0)
    ).map_err(|e| {
        debug!("Error getting last insert ID: {:?}", e);
        FluentError::from(e)
    })?;

    debug!("Inserted interaction with ID: {}", last_id);
    Ok(last_id)
}

pub fn log_request(conn: &Connection, flow_name: &str, request: &str, engine: &str) -> FluentResult<i64> {
    conn.execute(
        "INSERT INTO requests (flow_name, request, engine) VALUES (?, ?, ?)",
        params![flow_name, request, engine]
    ).map_err(|e| {
        debug!("Error inserting request: {:?}", e);
        FluentError::from(e)
    })?;

    // Retrieve the last inserted ID
    let last_id: i64 = conn.query_row(
        "SELECT lastval()",
        [],
        |row| row.get(0)
    ).map_err(|e| {
        debug!("Error getting last insert ID: {:?}", e);
        FluentError::from(e)
    })?;

    debug!("Inserted request with ID: {}", last_id);
    Ok(last_id)
}


pub async fn log_response(conn: &Connection, request_id: i64, response: &str, duration: f64) -> FluentResult<()> {
    debug!("Logging response");
    let mut stmt = conn.prepare(
        "INSERT INTO responses (request_id, response, duration) VALUES (?, ?, ?)"
    ).map_err(FluentError::from)?;
    stmt.execute([&request_id.to_string(), response, &duration.to_string()]).map_err(FluentError::from)?;
    debug!("Logged response");
    Ok(())
}

fn get_last_insert_id(conn: &Connection) -> FluentResult<i64> {
    let last_id: i64 = conn.query_row(
        "SELECT currval('id_sequence')",
        [],
        |row| row.get(0)
    ).map_err(|e| {
        debug!("Error getting last insert ID: {:?}", e);
        FluentError::from(e)
    })?;

    debug!("Inserted interaction with ID: {}", last_id);
    Ok(last_id)
}


use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use numpy::{PyArray1, PyArray2};
use serde_json;

pub fn log_anthropic_interaction(
    conn: &Connection,
    flow_name: &str,
    flow_config: &FlowConfig,
    request: &str,
    response: &AnthropicResponse,
    engine: &str,
    duration: f64
) -> FluentResult<i64> {
    let content = response.content.iter()
        .filter_map(|block| block.text.as_ref())
        .cloned()
        .collect::<Vec<String>>()
        .join("");

    let content_clone = content.clone();
    let flow_config_json = serde_json::to_string(flow_config)
        .map_err(|e| FluentError::CustomError(format!("Failed to serialize flow config: {}", e)))?;

    // Generate embedding using Spacy
    let embedding = Python::with_gil(|py| -> PyResult<Vec<f32>> {
        let spacy = py.import("spacy")?;
        let nlp = spacy.call_method1("load", ("en_core_web_sm",))?;
        let doc = nlp.call_method1("__call__", (content_clone,))?;
        let vector = doc.getattr("vector")?;
        let numpy_array: &PyArray1<f32> = vector.extract()?;
        Ok(numpy_array.to_vec()?)
    }).map_err(|e| FluentError::CustomError(format!("Failed to generate Spacy embedding: {:?}", e)))?;

    let embedding_json = serde_json::to_string(&embedding)
        .map_err(|e| FluentError::CustomError(format!("Failed to serialize embedding: {}", e)))?;

    conn.execute(
        "INSERT INTO interactions (
            flow_name, flow_config, request, response, embedding, engine, duration,
            anthropic_id, anthropic_type, anthropic_role, anthropic_model,
            anthropic_stop_reason, anthropic_stop_sequence,
            anthropic_input_tokens, anthropic_output_tokens
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            flow_name, flow_config_json, request, content, embedding_json, engine, duration,
            &response.id, &response.response_type, &response.role, &response.model,
            &response.stop_reason, &response.stop_sequence,
            response.usage.as_ref().map(|u| u.input_tokens as i64),
            response.usage.as_ref().map(|u| u.output_tokens as i64)
        ]
    ).map_err(|e| {
        debug!("Error inserting Anthropic interaction: {:?}", e);
        FluentError::from(e)
    })?;

    get_last_insert_id(conn)
}

use pyo3::prelude::*;



use pyo3::prelude::*;


pub fn log_openai_agent_interaction(
    conn: &Connection,
    flow_name: &str,
    flow_config: &FlowConfig,
    request: &str,
    response: &OpenAIResponse,
    engine: &str,
    duration: f64,
    thread_data: Option<&ThreadData>  // New parameter for Assistant thread data
) -> FluentResult<i64> {
    let content = response.choices.as_ref()
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.message.as_ref())
        .map(|message| message.content.clone())
        .unwrap_or_default();

    let content_clone = content.clone();
    let flow_config_json = serde_json::to_string(flow_config)
        .map_err(|e| FluentError::CustomError(format!("Failed to serialize flow config: {}", e)))?;

    // Generate embedding using Spacy
    let embedding = Python::with_gil(|py| -> PyResult<Vec<f32>> {
        let spacy = py.import("spacy")?;
        let nlp = spacy.call_method1("load", ("en_core_web_sm",))?;
        let doc = nlp.call_method1("__call__", (content_clone,))?;
        let vector = doc.getattr("vector")?;
        let numpy_array: &PyArray1<f32> = vector.extract()?;
        Ok(numpy_array.to_vec()?)
    }).map_err(|e| FluentError::CustomError(format!("Failed to generate Spacy embedding: {:?}", e)))?;

    let embedding_json = serde_json::to_string(&embedding)
        .map_err(|e| FluentError::CustomError(format!("Failed to serialize embedding: {}", e)))?;

    conn.execute(
        "INSERT INTO interactions (
            flow_name, flow_config, request, response, embedding, engine, duration,
            openai_id, openai_object, openai_created, openai_model, openai_finish_reason,
            openai_prompt_tokens, openai_completion_tokens, openai_total_tokens,
            openai_assistant_id, openai_thread_id, openai_run_id, openai_message_id,
            openai_role, openai_content, openai_created_at, openai_assistant_metadata,
            openai_thread_metadata
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            flow_name, flow_config_json, request, content, embedding_json, engine, duration,
            response.id.as_deref().unwrap_or(""),
            response.object.as_deref().unwrap_or(""),
            response.created.unwrap_or(0) as i64,
            response.model.as_deref().unwrap_or(""),
            response.choices.as_ref().and_then(|c| c.first()).and_then(|c| c.finish_reason.as_deref()).unwrap_or(""),
            response.usage.as_ref().map(|u| u.prompt_tokens as i64).unwrap_or(0),
            response.usage.as_ref().map(|u| u.completion_tokens as i64).unwrap_or(0),
            response.usage.as_ref().map(|u| u.total_tokens as i64).unwrap_or(0),
            thread_data.and_then(|d| d.assistant_id.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.thread_id.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.run_id.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.message_id.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.role.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.content.as_deref()).unwrap_or(""),
            thread_data.map(|d| d.created_at as i64).unwrap_or(0),
            thread_data.and_then(|d| d.assistant_metadata.as_deref()).unwrap_or(""),
            thread_data.and_then(|d| d.thread_metadata.as_deref()).unwrap_or("")
        ]
    ).map_err(|e| {
        debug!("Error inserting OpenAI interaction: {:?}", e);
        FluentError::from(e)
    })?;

    get_last_insert_id(conn)
}