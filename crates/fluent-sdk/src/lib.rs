pub mod ai;
pub mod pipeline;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Deserialize, Serialize, Clone, Display)]
#[allow(non_camel_case_types)]
pub enum Credential {
    STABILITYAI_API_KEY,
    OPENAI_API_KEY,
    ANTHROPIC_API_KEY,
    MISTRAL_API_KEY,
    GEMINI_API_KEY,
    GEMMA_API_KEY,
    GOOGLE_API_KEY,
    COHERE_API_KEY,
    LLAMA3_API_KEY,
    IMAGINEPRO_API_KEY,
    LEONARDO_API_KEY,
    PERPLEXITY_API_KEY,
}

pub mod prelude {
    pub use crate::ai::cohere::*;
    pub use crate::ai::flowise_sonnet_chain::*;
    pub use crate::ai::gemini_flash::*;
    pub use crate::ai::gemini_pro::*;
    pub use crate::ai::gemma_groq::*;
    pub use crate::ai::imaginepro::*;
    pub use crate::ai::leonardo::*;
    pub use crate::ai::llama3_groq::*;
    pub use crate::ai::mistral_large2::*;
    pub use crate::ai::mistral_nemo::*;
    pub use crate::ai::openai::*;
    pub use crate::ai::openai_dalle::*;
    pub use crate::ai::perplexity::*;
    pub use crate::ai::sonnet35::*;
    pub use crate::ai::stability_ultravertical::*;
    pub use crate::ai::{FluentRequest, FluentSdkRequest, KeyValue};
    pub use crate::pipeline::adapters::*;
    pub use crate::pipeline::model::*;
    pub use fluent_pipeline::*;
}
