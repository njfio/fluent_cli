use anyhow::Result;
use async_trait::async_trait;

use fluent_core::config::EngineConfig;
use fluent_core::traits::Engine;

#[async_trait]
pub trait EnginePlugin: Send + Sync {
    fn engine_type(&self) -> &str;

    async fn create(&self, config: EngineConfig) -> Result<Box<dyn Engine>>;
}

// SECURITY: Plugin system is disabled for safety reasons
//
// The previous plugin system had FFI safety issues including:
// - Unsafe dynamic library loading
// - Unvalidated function pointers
// - Memory safety violations
// - Lack of sandboxing
//
// TODO: Implement a secure plugin system with:
// 1. WebAssembly-based sandboxing (WASI)
// 2. Capability-based security model
// 3. Memory isolation
// 4. Resource limits and quotas
// 5. Cryptographic signature verification
// 6. Audit logging
// 7. Permission system
//
// For now, all engines are statically compiled for security.

// Placeholder types for future secure implementation
// These are safe function pointers that don't involve FFI
pub type CreateEngineFn = fn() -> ();
pub type EngineTypeFn = fn() -> &'static str;

// Note: Any future plugin implementation should:
// - Never use `unsafe` blocks without extensive documentation
// - Validate all inputs from plugins
// - Use memory-safe interfaces only
// - Implement proper error boundaries
// - Include comprehensive security testing

