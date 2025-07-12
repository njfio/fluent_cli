//! Modular reflection system components
//! 
//! This module contains the refactored reflection system organized into
//! focused, single-responsibility modules for better maintainability.

pub mod analysis;
pub mod learning;
pub mod recommendations;
pub mod strategy;
pub mod types;

// Re-export commonly used types and functions
pub use analysis::{
    BottleneckDetector, LearningAnalyzer, ProgressAnalyzer, ResourceAnalyzer, StrategyEvaluator,
};
pub use learning::{
    KnowledgeRetentionManager, LearningInsightExtractor, PatternRecognizer,
};
pub use recommendations::{RecommendationGenerator, RecommendationPrioritizer};
pub use strategy::{StrategyAdjustmentGenerator, StrategyOptimizer};
pub use types::*;
