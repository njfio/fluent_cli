use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum::Display;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentOpenAIDalleRequest {}

/*
The style of the generated images. Must be one of vivid or natural. Vivid causes the model to lean towards generating hyper-real and dramatic images. Natural causes the model to produce more natural, less hyper-real looking images. This param is only supported for dall-e-3.
 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentOpeanAIStyle {
    #[serde(rename = "vivid")]
    #[strum(serialize = "vivid")]
    Vivid,
    #[serde(rename = "natural")]
    #[strum(serialize = "natural")]
    Natural,
}
/*
The quality of the image that will be generated. hd creates images with finer details and greater consistency across the image. This param is only supported for dall-e-3.
 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentOpenAIQuality {
    #[serde(rename = "hd")]
    #[strum(serialize = "hd")]
    Hd,
}

/*size
string or null

Optional
Defaults to 1024x1024
The size of the generated images. Must be one of 256x256, 512x512, or 1024x1024 for dall-e-2. Must be one of 1024x1024, 1792x1024, or 1024x1792 for dall-e-3 models.
 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentOpenAIDalleSize {
    #[serde(rename = "256x256")]
    #[strum(serialize = "256x256")]
    Size256x256,
    #[serde(rename = "512x512")]
    #[strum(serialize = "512x512")]
    Size512x512,
    #[serde(rename = "1024x1024")]
    #[strum(serialize = "1024x1024")]
    Size1024x1024,
    #[serde(rename = "1792x1024")]
    #[strum(serialize = "1792x1024")]
    Size1792x1024,
    #[serde(rename = "1024x1792")]
    #[strum(serialize = "1024x1792")]
    Size1024x1792,
}
/**
*         "sessionID": "NJF1234567DEFAULT",
       "n": 1,
       "logprobs": null,
       "echo": false,
       "user": "example-user-id",
       "size": "1024x1792",
       "style": "vivid",
       "quality": "hd"

*/
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentOpenAIDalleRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub model: Option<String>,
    pub n: Option<i8>,
    pub response_format: Option<Value>,
    pub logprobs: Option<i64>,
    pub echo: Option<bool>,
    pub user: Option<String>,
    pub size: Option<FluentOpenAIDalleSize>,
    pub style: Option<FluentOpeanAIStyle>,
    pub quality: Option<FluentOpenAIQuality>,
}
impl From<FluentOpenAIDalleRequest> for FluentRequest {
    fn from(request: FluentOpenAIDalleRequest) -> Self {
        let mut overrides = vec![];
        if let Some(response_format) = request.response_format {
            overrides.push(("response_format".to_string(), response_format));
        }
        if let Some(model) = request.model {
            overrides.push(("modelName".to_string(), json!(model)));
        }
        if let Some(n) = request.n {
            overrides.push(("n".to_string(), json!(n)));
        }
        if let Some(logprobs) = request.logprobs {
            overrides.push(("logprobs".to_string(), json!(logprobs)));
        }
        if let Some(echo) = request.echo {
            overrides.push(("echo".to_string(), json!(echo)));
        }
        if let Some(user) = request.user {
            overrides.push(("user".to_string(), json!(user)));
        }
        if let Some(size) = request.size {
            overrides.push(("size".to_string(), json!(size)));
        }
        if let Some(style) = request.style {
            overrides.push(("style".to_string(), json!(style)));
        }
        if let Some(quality) = request.quality {
            overrides.push(("quality".to_string(), json!(quality)));
        }
        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::OpenAiDalle),
            credentials: Some(vec![KeyValue::new("OPENAI_API_KEY", &request.bearer_token)]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentOpenAIDalleRequestBuilder {
    request: FluentOpenAIDalleRequest,
}
impl Default for FluentOpenAIDalleRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentOpenAIDalleRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                response_format: None,
                model: None,
                n: None,
                logprobs: None,
                echo: None,
                user: None,
                size: None,
                style: None,
                quality: None,
            },
        }
    }
}

impl FluentOpenAIDalleRequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.bearer_token = bearer_token;
        self
    }
    pub fn response_format(mut self, response_format: Value) -> Self {
        self.request.response_format = Some(response_format);
        self
    }
    pub fn model(mut self, model: String) -> Self {
        self.request.model = Some(model);
        self
    }
    pub fn n(mut self, n: i8) -> Self {
        self.request.n = Some(n);
        self
    }
    pub fn logprobs(mut self, logprobs: i64) -> Self {
        self.request.logprobs = Some(logprobs);
        self
    }
    pub fn echo(mut self, echo: bool) -> Self {
        self.request.echo = Some(echo);
        self
    }
    pub fn user(mut self, user: String) -> Self {
        self.request.user = Some(user);
        self
    }
    pub fn size(mut self, size: FluentOpenAIDalleSize) -> Self {
        self.request.size = Some(size);
        self
    }
    pub fn style(mut self, style: FluentOpeanAIStyle) -> Self {
        self.request.style = Some(style);
        self
    }
    pub fn quality(mut self, quality: FluentOpenAIQuality) -> Self {
        self.request.quality = Some(quality);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentOpenAIDalleRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("OpenAI key is required"));
        }
        Ok(self.request)
    }
}
