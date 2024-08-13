use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::Display;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentLeonardoRequest {}

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentLeonardoPromptMagicVersion {
    #[serde(rename = "v2")]
    #[strum(serialize = "v2")]
    V2,
    #[serde(rename = "v3")]
    #[strum(serialize = "v3")]
    V3,
}

/*
The base version of stable diffusion to use if not using a custom model. v1_5 is 1.5, v2 is 2.1, if not specified it will default to v1_5. Also includes SDXL and SDXL Lightning models
*/
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentLeonardoSdVersion {
    #[serde(rename = "v1_5")]
    #[strum(serialize = "v1_5")]
    V1_5,
    #[serde(rename = "v2")]
    #[strum(serialize = "v2")]
    V2,
}

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentLeonardoPresetStyle {
    #[serde(rename = "LEONARDO")]
    #[strum(serialize = "LEONARDO")]
    Leonardo,
    #[serde(rename = "NONE")]
    #[strum(serialize = "NONE")]
    None,
    #[serde(rename = "ANIME")]
    #[strum(serialize = "ANIME")]
    Anime,
    #[serde(rename = "CREATIVE")]
    #[strum(serialize = "CREATIVE")]
    Creative,
    #[serde(rename = "DYNAMIC")]
    #[strum(serialize = "DYNAMIC")]
    Dynamic,
    #[serde(rename = "ENVIRONMENT")]
    #[strum(serialize = "ENVIRONMENT")]
    Environment,
    #[serde(rename = "GENERAL")]
    #[strum(serialize = "GENERAL")]
    General,
    #[serde(rename = "ILLUSTRATION")]
    #[strum(serialize = "ILLUSTRATION")]
    Illustration,
    #[serde(rename = "PHOTOGRAPHY")]
    #[strum(serialize = "PHOTOGRAPHY")]
    Photography,
    #[serde(rename = "RAYTRACED")]
    #[strum(serialize = "RAYTRACED")]
    Raytraced,
    #[serde(rename = "RENDER_3D")]
    #[strum(serialize = "RENDER_3D")]
    Render3D,
    #[serde(rename = "SKETCH_BW")]
    #[strum(serialize = "SKETCH_BW")]
    SketchBW,
    #[serde(rename = "SKETCH_COLOR")]
    #[strum(serialize = "SKETCH_COLOR")]
    SketchColor,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentLeonardoRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub model_id: Option<String>,
    pub steps: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub prompt_magic: Option<bool>,
    pub num_images: Option<i64>,
    pub nsfw: Option<bool>,
    pub public: Option<bool>,
    pub negative_prompt: Option<String>,
    pub guidance_scale: Option<i64>,
    pub prompt_magic_version: Option<FluentLeonardoPromptMagicVersion>,
    pub prompt_magic_strength: Option<f64>,
    pub preset_style: Option<FluentLeonardoPresetStyle>,
    pub high_resolution: Option<bool>,
    pub high_contrast: Option<bool>,
    pub alchemy: Option<bool>,
    pub photo_real: Option<bool>,
    pub tiling: Option<bool>,
    pub weighting: Option<i64>,
    pub sd_version: Option<FluentLeonardoSdVersion>,
}

impl From<FluentLeonardoRequest> for FluentRequest {
    fn from(request: FluentLeonardoRequest) -> Self {
        let mut overrides = vec![];
        if let Some(steps) = request.steps {
            overrides.push(("num_inference_steps".to_string(), json!(steps)));
        }
        if let Some(width) = request.width {
            overrides.push(("width".to_string(), json!(width)));
        }
        if let Some(height) = request.height {
            overrides.push(("height".to_string(), json!(height)));
        }
        if let Some(prompt_magic) = request.prompt_magic {
            overrides.push(("promptMagic".to_string(), json!(prompt_magic)));
        }
        if let Some(num_images) = request.num_images {
            overrides.push(("num_images".to_string(), json!(num_images)));
        }
        if let Some(nsfw) = request.nsfw {
            overrides.push(("nsfw".to_string(), json!(nsfw)));
        }
        if let Some(public) = request.public {
            overrides.push(("public".to_string(), json!(public)));
        }
        if let Some(negative_prompt) = request.negative_prompt {
            overrides.push(("negative_prompt".to_string(), json!(negative_prompt)));
        }
        if let Some(guidance_scale) = request.guidance_scale {
            overrides.push(("guidance_scale".to_string(), json!(guidance_scale)));
        }
        if let Some(prompt_magic_version) = request.prompt_magic_version {
            overrides.push((
                "promptMagicVersion".to_string(),
                json!(prompt_magic_version),
            ));
        }
        if let Some(prompt_magic_strength) = request.prompt_magic_strength {
            overrides.push((
                "promptMagicStrength".to_string(),
                json!(prompt_magic_strength),
            ));
        }
        if let Some(preset_style) = request.preset_style {
            overrides.push(("presetStyle".to_string(), json!(preset_style)));
        }
        if let Some(high_resolution) = request.high_resolution {
            overrides.push(("highResolution".to_string(), json!(high_resolution)));
        }
        if let Some(high_contrast) = request.high_contrast {
            overrides.push(("highContrast".to_string(), json!(high_contrast)));
        }
        if let Some(alchemy) = request.alchemy {
            overrides.push(("alchemy".to_string(), json!(alchemy)));
        }
        if let Some(photo_real) = request.photo_real {
            overrides.push(("photoReal".to_string(), json!(photo_real)));
        }
        if let Some(tiling) = request.tiling {
            overrides.push(("tiling".to_string(), json!(tiling)));
        }
        if let Some(weighting) = request.weighting {
            overrides.push(("weighting".to_string(), json!(weighting)));
        }
        if let Some(sd_version) = request.sd_version {
            overrides.push(("sd_version".to_string(), json!(sd_version)));
        }
        if let Some(model_id) = request.model_id {
            overrides.push(("modelId".to_string(), json!(model_id)));
        }

        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::Leonardo),
            credentials: Some(vec![KeyValue::new(
                "LEONARDO_API_KEY",
                &request.bearer_token,
            )]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentLeonardoRequestBuilder {
    request: FluentLeonardoRequest,
}
impl Default for FluentLeonardoRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentLeonardoRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                steps: None,
                width: None,
                height: None,
                prompt_magic: None,
                num_images: None,
                nsfw: None,
                public: None,
                negative_prompt: None,
                guidance_scale: None,
                prompt_magic_version: None,
                prompt_magic_strength: None,
                preset_style: None,
                high_resolution: None,
                high_contrast: None,
                alchemy: None,
                photo_real: None,
                tiling: None,
                weighting: None,
                sd_version: None,
                model_id: None,
            },
        }
    }
}

impl FluentLeonardoRequestBuilder {
    pub fn prompt(mut self, prompt: String) -> Self {
        self.request.prompt = prompt;
        self
    }
    pub fn bearer_token(mut self, bearer_token: String) -> Self {
        self.request.bearer_token = bearer_token;
        self
    }
    pub fn steps(mut self, steps: i64) -> Self {
        self.request.steps = Some(steps);
        self
    }
    pub fn width(mut self, width: i64) -> Self {
        self.request.width = Some(width);
        self
    }
    pub fn height(mut self, height: i64) -> Self {
        self.request.height = Some(height);
        self
    }
    pub fn prompt_magic(mut self, prompt_magic: bool) -> Self {
        self.request.prompt_magic = Some(prompt_magic);
        self
    }
    pub fn num_images(mut self, num_images: i64) -> Self {
        self.request.num_images = Some(num_images);
        self
    }
    pub fn nsfw(mut self, nsfw: bool) -> Self {
        self.request.nsfw = Some(nsfw);
        self
    }
    pub fn public(mut self, public: bool) -> Self {
        self.request.public = Some(public);
        self
    }
    pub fn negative_prompt(mut self, negative_prompt: String) -> Self {
        self.request.negative_prompt = Some(negative_prompt);
        self
    }
    pub fn guidance_scale(mut self, guidance_scale: i64) -> Self {
        self.request.guidance_scale = Some(guidance_scale);
        self
    }
    pub fn prompt_magic_version(
        mut self,
        prompt_magic_version: FluentLeonardoPromptMagicVersion,
    ) -> Self {
        self.request.prompt_magic_version = Some(prompt_magic_version);
        self
    }
    pub fn prompt_magic_strength(mut self, prompt_magic_strength: f64) -> Self {
        self.request.prompt_magic_strength = Some(prompt_magic_strength);
        self
    }
    pub fn preset_style(mut self, preset_style: FluentLeonardoPresetStyle) -> Self {
        self.request.preset_style = Some(preset_style);
        self
    }
    pub fn high_resolution(mut self, high_resolution: bool) -> Self {
        self.request.high_resolution = Some(high_resolution);
        self
    }
    pub fn high_contrast(mut self, high_contrast: bool) -> Self {
        self.request.high_contrast = Some(high_contrast);
        self
    }
    pub fn alchemy(mut self, alchemy: bool) -> Self {
        self.request.alchemy = Some(alchemy);
        self
    }
    pub fn photo_real(mut self, photo_real: bool) -> Self {
        self.request.photo_real = Some(photo_real);
        self
    }
    pub fn tiling(mut self, tiling: bool) -> Self {
        self.request.tiling = Some(tiling);
        self
    }
    pub fn weighting(mut self, weighting: i64) -> Self {
        self.request.weighting = Some(weighting);
        self
    }
    pub fn sd_version(mut self, sd_version: FluentLeonardoSdVersion) -> Self {
        self.request.sd_version = Some(sd_version);
        self
    }
    pub fn model_id(mut self, model_id: String) -> Self {
        self.request.model_id = Some(model_id);
        self
    }
    pub fn build(self) -> anyhow::Result<FluentLeonardoRequest> {
        if self.request.prompt.is_empty() {
            return Err(anyhow!("Prompt is required"));
        }
        if self.request.bearer_token.is_empty() {
            return Err(anyhow!("Bearer Token is required"));
        }
        if let Some(width) = self.request.width {
            if width % 64 != 0 {
                return Err(anyhow!("Width must be a multiple of 64"));
            }
            if width < 128 {
                return Err(anyhow!("Width must be at least 128"));
            }
        }
        if let Some(height) = self.request.height {
            if height % 64 != 0 {
                return Err(anyhow!("Height must be a multiple of 64"));
            }
            if height < 128 {
                return Err(anyhow!("Height must be at least 128"));
            }
        }
        Ok(self.request)
    }
}
