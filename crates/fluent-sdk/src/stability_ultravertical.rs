use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;
use strum::Display;

use crate::{EngineName, FluentRequest, FluentSdkRequest, KeyValue};

impl FluentSdkRequest for FluentStabilityRequest {}

//3d-model analog-film anime cinematic comic-book digital-art enhance fantasy-art isometric line-art low-poly modeling-compound neon-punk origami photographic pixel-art tile-texture
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentStabilityStylePreset {
    #[serde(rename = "3d-model")]
    #[strum(to_string = "3d-model", ascii_case_insensitive)]
    Model3D,
    #[serde(rename = "analog-film")]
    #[strum(to_string = "analog-film", ascii_case_insensitive)]
    AnalogFilm,
    #[serde(rename = "anime")]
    #[strum(to_string = "anime", ascii_case_insensitive)]
    Anime,
    #[serde(rename = "cinematic")]
    #[strum(to_string = "cinematic", ascii_case_insensitive)]
    Cinematic,
    #[serde(rename = "comic-book")]
    #[strum(to_string = "comic-book", ascii_case_insensitive)]
    ComicBook,
    #[serde(rename = "digital-art")]
    #[strum(to_string = "digital-art", ascii_case_insensitive)]
    DigitalArt,
    #[serde(rename = "enhance")]
    #[strum(to_string = "enhance", ascii_case_insensitive)]
    Enhance,
    #[serde(rename = "fantasy-art")]
    #[strum(to_string = "fantasy-art", ascii_case_insensitive)]
    FantasyArt,
    #[serde(rename = "isometric")]
    #[strum(to_string = "isometric", ascii_case_insensitive)]
    Isometric,
    #[serde(rename = "line-art")]
    #[strum(to_string = "line-art", ascii_case_insensitive)]
    LineArt,
    #[serde(rename = "low-poly")]
    #[strum(to_string = "low-poly", ascii_case_insensitive)]
    LowPoly,
    #[serde(rename = "modeling-compound")]
    #[strum(to_string = "modeling-compound", ascii_case_insensitive)]
    ModelingCompound,
    #[serde(rename = "neon-punk")]
    #[strum(to_string = "neon-punk", ascii_case_insensitive)]
    NeonPunk,
    #[serde(rename = "origami")]
    #[strum(to_string = "origami", ascii_case_insensitive)]
    Origami,
    #[serde(rename = "photographic")]
    #[strum(to_string = "photographic", ascii_case_insensitive)]
    Photographic,
    #[serde(rename = "pixel-art")]
    #[strum(to_string = "pixel-art", ascii_case_insensitive)]
    PixelArt,
    #[serde(rename = "tile-texture")]
    #[strum(to_string = "tile-texture", ascii_case_insensitive)]
    TileTexture,
}
/*
string
Default: 1:1
Enum: 16:9 1:1 21:9 2:3 3:2 4:5 5:4 9:16 9:21 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentStabilityAspectoRatio {
    #[serde(rename = "16:9")]
    #[strum(to_string = "16:9", ascii_case_insensitive)]
    Aspect16_9,
    #[serde(rename = "1:1")]
    #[strum(to_string = "1:1", ascii_case_insensitive)]
    Aspect1_1,
    #[serde(rename = "21:9")]
    #[strum(to_string = "21:9", ascii_case_insensitive)]
    Aspect21_9,
    #[serde(rename = "2:3")]
    #[strum(to_string = "2:3", ascii_case_insensitive)]
    Aspect2_3,
    #[serde(rename = "3:2")]
    #[strum(to_string = "3:2", ascii_case_insensitive)]
    Aspect3_2,
    #[serde(rename = "4:5")]
    #[strum(to_string = "4:5", ascii_case_insensitive)]
    Aspect4_5,
    #[serde(rename = "5:4")]
    #[strum(to_string = "5:4", ascii_case_insensitive)]
    Aspect5_4,
    #[serde(rename = "9:16")]
    #[strum(to_string = "9:16", ascii_case_insensitive)]
    Aspect9_16,
    #[serde(rename = "9:21")]
    #[strum(to_string = "9:21", ascii_case_insensitive)]
    Aspect9_21,
}

/*
Default: png
Enum: jpeg png webp
 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
pub enum FluentStabilityOutputFormat {
    #[serde(rename = "jpeg")]
    #[strum(to_string = "jpeg", ascii_case_insensitive)]
    Jpeg,
    #[serde(rename = "png")]
    #[strum(to_string = "png", ascii_case_insensitive)]
    Png,
    #[serde(rename = "webp")]
    #[strum(to_string = "webp", ascii_case_insensitive)]
    Webp,
}

/*
Enum: DDIM DDPM K_DPMPP_2M K_DPMPP_2S_ANCESTRAL K_DPM_2 K_DPM_2_ANCESTRAL K_EULER K_EULER_ANCESTRAL K_HEUN K_LMS
 */
#[derive(Debug, Deserialize, Serialize, Clone, Display)]
#[allow(non_camel_case_types)]
pub enum FluentStabilitySampler {
    DDIM,
    DDPM,
    K_DPMPP_2M,
    K_DPMPP_2S_ANCESTRAL,
    K_DPM_2,
    K_DPM_2_ANCESTRAL,
    K_EULER,
    K_EULER_ANCESTRAL,
    K_HEUN,
    K_LMS,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentStabilityRequest {
    pub prompt: String,
    pub bearer_token: String,
    pub steps: Option<i64>,
    pub cfg_scale: Option<i64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub samples: Option<i64>,
    pub seed: Option<i64>,
    pub style_preset: Option<FluentStabilityStylePreset>,
    pub aspect_ratio: Option<FluentStabilityAspectoRatio>,
    pub output_format: Option<FluentStabilityOutputFormat>,
    pub sampler: Option<FluentStabilitySampler>,
}

impl From<FluentStabilityRequest> for FluentRequest {
    fn from(request: FluentStabilityRequest) -> Self {
        let mut overrides = vec![];
        if let Some(steps) = request.steps {
            overrides.push(("steps".to_string(), json!(steps)));
        }
        if let Some(cfg_scale) = request.cfg_scale {
            overrides.push(("cfg_scale".to_string(), json!(cfg_scale)));
        }
        if let Some(width) = request.width {
            overrides.push(("width".to_string(), json!(width)));
        }
        if let Some(height) = request.height {
            overrides.push(("height".to_string(), json!(height)));
        }
        if let Some(samples) = request.samples {
            overrides.push(("samples".to_string(), json!(samples)));
        }
        if let Some(seed) = request.seed {
            overrides.push(("seed".to_string(), json!(seed)));
        }
        if let Some(style_preset) = request.style_preset {
            overrides.push(("style_preset".to_string(), json!(style_preset.to_string())));
        }
        if let Some(aspect_ratio) = request.aspect_ratio {
            overrides.push(("aspect_ratio".to_string(), json!(aspect_ratio.to_string())));
        }
        if let Some(output_format) = request.output_format {
            overrides.push((
                "output_format".to_string(),
                json!(output_format.to_string()),
            ));
        }
        if let Some(sampler) = request.sampler {
            overrides.push(("sampler".to_string(), json!(sampler.to_string())));
        }

        FluentRequest {
            request: Some(request.prompt),
            engine: Some(EngineName::StabilityUltraVertical),
            credentials: Some(vec![KeyValue::new(
                "STABILITYAI_API_KEY",
                &request.bearer_token,
            )]),
            overrides: Some(overrides.into_iter().collect()),
            parse_code: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FluentStabilityRequestBuilder {
    request: FluentStabilityRequest,
}
impl Default for FluentStabilityRequestBuilder {
    fn default() -> Self {
        Self {
            request: FluentStabilityRequest {
                prompt: String::new(),
                bearer_token: String::new(),
                steps: None,
                cfg_scale: None,
                width: None,
                height: None,
                samples: None,
                seed: None,
                style_preset: None,
                aspect_ratio: None,
                output_format: None,
                sampler: None,
            },
        }
    }
}

impl FluentStabilityRequestBuilder {
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
    pub fn cfg_scale(mut self, cfg_scale: i64) -> Self {
        self.request.cfg_scale = Some(cfg_scale);
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
    pub fn samples(mut self, samples: i64) -> Self {
        self.request.samples = Some(samples);
        self
    }
    pub fn seed(mut self, seed: i64) -> Self {
        self.request.seed = Some(seed);
        self
    }
    pub fn style_preset(mut self, style_preset: FluentStabilityStylePreset) -> Self {
        self.request.style_preset = Some(style_preset);
        self
    }
    pub fn aspect_ratio(mut self, aspect_ratio: FluentStabilityAspectoRatio) -> Self {
        self.request.aspect_ratio = Some(aspect_ratio);
        self
    }
    pub fn output_format(mut self, output_format: FluentStabilityOutputFormat) -> Self {
        self.request.output_format = Some(output_format);
        self
    }
    pub fn sampler(mut self, sampler: FluentStabilitySampler) -> Self {
        self.request.sampler = Some(sampler);
        self
    }

    pub fn build(self) -> anyhow::Result<FluentStabilityRequest> {
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
