//! Pipeline execution modules
//! 
//! This module contains the refactored pipeline execution functionality,
//! broken down into focused, single-responsibility modules.

pub mod step_executor;
pub mod command_executor;
pub mod parallel_executor;
pub mod condition_executor;
pub mod loop_executor;
pub mod variable_expander;

// Re-export key types and functions for easy access
pub use step_executor::StepExecutor;
pub use command_executor::CommandExecutor;
pub use parallel_executor::ParallelExecutor;
pub use condition_executor::ConditionExecutor;
pub use loop_executor::LoopExecutor;
pub use variable_expander::VariableExpander;
