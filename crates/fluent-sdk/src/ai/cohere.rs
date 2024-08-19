use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Value};
use strum::Display;

use super::{EngineName, FluentRequest, KeyValue};

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentCoherePromptTruncation {
    #[serde(rename = "AUTO")]
    #[strum(to_string = "AUTO", ascii_case_insensitive)]
    Auto,
    #[serde(rename = "AUTO_PRESERVE_ORDER")]
    #[strum(to_string = "AUTO_PRESERVE_ORDER", ascii_case_insensitive)]
    AutoPreserveOrder,
    #[serde(rename = "OFF")]
    #[strum(to_string = "OFF", ascii_case_insensitive)]
    Off,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FluentCohereConnector {
    WebSearch,
    Custom(String),
}
impl Serialize for FluentCohereConnector {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            FluentCohereConnector::WebSearch => serializer.serialize_str("web-search"),
            FluentCohereConnector::Custom(value) => serializer.serialize_str(value),
        }
    }
}
impl<'de> Deserialize<'de> for FluentCohereConnector {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "web-search" => Ok(FluentCohereConnector::WebSearch),
            _ => Ok(FluentCohereConnector::Custom(s)),
        }
    }
}
impl FromStr for FluentCohereConnector {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web-search" => Ok(FluentCohereConnector::WebSearch),
            _ => Ok(FluentCohereConnector::Custom(s.to_string())),
        }
    }
}
impl Display for FluentCohereConnector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FluentCohereConnector::WebSearch => write!(f, "web-search"),
            FluentCohereConnector::Custom(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentCohereCitationQuality {
    #[serde(rename = "accurate")]
    #[strum(to_string = "accurate", ascii_case_insensitive)]
    Accurate,
    #[serde(rename = "fast")]
    #[strum(to_string = "fast", ascii_case_insensitive)]
    Fast,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentCohereRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub top_k: Option<i64>,
    pub top_p: Option<f64>,
    pub session_id: Option<String>,
    pub frequency_penalty: Option<f64>,
    pub presence_penalty: Option<f64>,
    pub preamble: Option<String>,
    pub conversation_id: Option<String>,
    pub prompt_truncation: Option<FluentCoherePromptTruncation>,
    pub connectors: Option<Vec<FluentCohereConnector>>,
    pub citation_quality: Option<FluentCohereCitationQuality>,
}

impl From<FluentCohereRequest> for FluentRequest {
    fn from(request: FluentCohereRequest) -> Self {
        let mut overrides = vec![];
        if let Some(temperature) = request.temperature {
            overrides.push(("temperature".to_string(), json!(temperature)));
        }
        if let Some(max_tokens) = request.max_tokens {
            overrides.push(("max_tokens".to_string(), json!(max_tokens)));
        }
        if let Some(model_name) = request.model {
            overrides.push(("modelName".to_string(), json!(model_name)));
        }
        if let Some(top_k) = request.top_k {
            overrides.push(("top_k".to_string(), json!(top_k)));
        }
        if let Some(top_p) = request.top_p {
            overrides.push(("top_p".to_string(), json!(top_p)));
        }
        if let Some(session_id) = request.session_id {
            overrides.push(("sessionId".to_string(), json!(session_id)));
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            overrides.push(("frequency_penalty".to_string(), json!(frequency_penalty)));
        }
        if let Some(presence_penalty) = request.presence_penalty {
            overrides.push(("presence_penalty".to_string(), json!(presence_penalty)));
        }
        if let Some(preamble) = request.preamble {
            overrides.push(("preamble".to_string(), json!(preamble)));
        }
        if let Some(conversation_id) = request.conversation_id {
            overrides.push(("conversation_id".to_string(), json!(conversation_id)));
        }
        if let Some(prompt_truncation) = request.prompt_truncation {
            overrides.push(("prompt_truncation".to_string(), json!(prompt_truncation)));
        }
        if let Some(connectors) = request.connectors {
            overrides.push((
                "connectors".to_string(),
                Value::Array(
                    connectors
                        .into_iter()
                        .map(|c| {
                            json! {
                                { "id": c.to_string() }
                            }
                        })
                        .collect::<Vec<_>>(),
                ),
            ));
        }
        if let Some(citation_quality) = request.citation_quality {
            overrides.push(("citation_quality".to_string(), json!(citation_quality)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::Cohere),
            credentials: Some(vec![KeyValue::new("COHERE_API_KEY", &request.bearer_token)]),
            overrides: Some(overrides.into_iter().collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentCohereRequestBuilder {
    request: FluentCohereRequest,
}
impl Default for FluentCohereRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentCohereRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                model: None,
                temperature: None,
                max_tokens: None,
                top_k: None,
                top_p: None,
                session_id: None,
                frequency_penalty: None,
                presence_penalty: None,
                preamble: None,
                conversation_id: None,
                prompt_truncation: None,
                connectors: None,
                citation_quality: None,
            },
        }
    }
}

impl FluentCohereRequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.bearer_token = bearer_token;
        self
    }
    pub fn temperature(mut self, temperature: f64) -> Self {
        self.request.temperature = Some(temperature);
        self
    }
    pub fn max_tokens(mut self, max_tokens: i64) -> Self {
        self.request.max_tokens = Some(max_tokens);
        self
    }
    pub fn model(mut self, model: String) -> Self {
        self.request.model = Some(model);
        self
    }
    pub fn top_k(mut self, top_k: i64) -> Self {
        self.request.top_k = Some(top_k);
        self
    }
    pub fn top_p(mut self, top_p: f64) -> Self {
        self.request.top_p = Some(top_p);
        self
    }
    pub fn session_id(mut self, session_id: String) -> Self {
        self.request.session_id = Some(session_id);
        self
    }
    pub fn frequency_penalty(mut self, frequency_penalty: f64) -> Self {
        self.request.frequency_penalty = Some(frequency_penalty);
        self
    }
    pub fn presence_penalty(mut self, presence_penalty: f64) -> Self {
        self.request.presence_penalty = Some(presence_penalty);
        self
    }
    pub fn preamble(mut self, preamble: String) -> Self {
        self.request.preamble = Some(preamble);
        self
    }
    pub fn conversation_id(mut self, conversation_id: String) -> Self {
        self.request.conversation_id = Some(conversation_id);
        self
    }
    pub fn prompt_truncation(mut self, prompt_truncation: FluentCoherePromptTruncation) -> Self {
        self.request.prompt_truncation = Some(prompt_truncation);
        self
    }
    pub fn connectors(mut self, connectors: Vec<FluentCohereConnector>) -> Self {
        self.request.connectors = Some(connectors);
        self
    }
    pub fn citation_quality(mut self, citation_quality: FluentCohereCitationQuality) -> Self {
        self.request.citation_quality = Some(citation_quality);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentCohereRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("Bearer Token is required"));
        }
        Ok(self.request)
    }
}
