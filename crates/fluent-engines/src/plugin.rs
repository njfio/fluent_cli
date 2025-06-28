use anyhow::Result;
use async_trait::async_trait;

use fluent_core::config::EngineConfig;
use fluent_core::traits::Engine;

#[async_trait]
pub trait EnginePlugin: Send + Sync {
    fn engine_type(&self) -> &str;

    async fn create(&self, config: EngineConfig) -> Result<Box<dyn Engine>>;
}

pub type CreateEngineFn = unsafe extern "C" fn(EngineConfig) -> Box<dyn Engine>;
pub type EngineTypeFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

