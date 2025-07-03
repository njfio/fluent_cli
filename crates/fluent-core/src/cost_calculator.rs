use anyhow::{anyhow, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Cost, Usage};

/// Pricing model for different engines and models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingModel {
    pub engine: String,
    pub model: String,
    pub prompt_rate: f64,        // Cost per 1K prompt tokens
    pub completion_rate: f64,    // Cost per 1K completion tokens
    pub image_rate: Option<f64>, // Cost per image (for image generation)
    pub last_updated: String,    // ISO 8601 timestamp
}

/// Cost calculation limits and validation
#[derive(Debug, Clone)]
pub struct CostLimits {
    pub max_single_request: f64, // Maximum cost for a single request
    pub max_daily_total: f64,    // Maximum daily cost limit
    pub warn_threshold: f64,     // Warning threshold for high costs
}

impl Default for CostLimits {
    fn default() -> Self {
        Self {
            max_single_request: 10.0, // $10 per request
            max_daily_total: 100.0,   // $100 per day
            warn_threshold: 1.0,      // Warn at $1
        }
    }
}

/// Secure cost calculator with validation and audit logging
pub struct CostCalculator {
    pricing_models: HashMap<String, PricingModel>,
    limits: CostLimits,
    daily_total: f64,
}

impl CostCalculator {
    pub fn new() -> Self {
        let mut calculator = Self {
            pricing_models: HashMap::new(),
            limits: CostLimits::default(),
            daily_total: 0.0,
        };

        calculator.load_default_pricing();
        calculator
    }

    pub fn with_limits(limits: CostLimits) -> Self {
        let mut calculator = Self::new();
        calculator.limits = limits;
        calculator
    }

    /// Load default pricing models for all supported engines
    fn load_default_pricing(&mut self) {
        // OpenAI pricing (as of 2024)
        self.add_pricing_model(PricingModel {
            engine: "openai".to_string(),
            model: "gpt-4o".to_string(),
            prompt_rate: 0.005,     // $5 per 1M tokens
            completion_rate: 0.015, // $15 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        self.add_pricing_model(PricingModel {
            engine: "openai".to_string(),
            model: "gpt-4".to_string(),
            prompt_rate: 0.01,     // $10 per 1M tokens
            completion_rate: 0.03, // $30 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        self.add_pricing_model(PricingModel {
            engine: "openai".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            prompt_rate: 0.0015,    // $1.50 per 1M tokens
            completion_rate: 0.002, // $2 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // Anthropic pricing
        self.add_pricing_model(PricingModel {
            engine: "anthropic".to_string(),
            model: "claude-3-opus".to_string(),
            prompt_rate: 0.015,     // $15 per 1M tokens
            completion_rate: 0.075, // $75 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        self.add_pricing_model(PricingModel {
            engine: "anthropic".to_string(),
            model: "claude-3-sonnet".to_string(),
            prompt_rate: 0.003,     // $3 per 1M tokens
            completion_rate: 0.015, // $15 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        self.add_pricing_model(PricingModel {
            engine: "anthropic".to_string(),
            model: "claude-3-haiku".to_string(),
            prompt_rate: 0.00025,     // $0.25 per 1M tokens
            completion_rate: 0.00125, // $1.25 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // Google Gemini pricing
        self.add_pricing_model(PricingModel {
            engine: "google_gemini".to_string(),
            model: "gemini-1.5-pro".to_string(),
            prompt_rate: 0.003,    // $3 per 1M tokens
            completion_rate: 0.01, // $10 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        self.add_pricing_model(PricingModel {
            engine: "google_gemini".to_string(),
            model: "gemini-1.5-flash".to_string(),
            prompt_rate: 0.0025,     // $2.50 per 1M tokens
            completion_rate: 0.0075, // $7.50 per 1M tokens
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // Cohere pricing (estimated)
        self.add_pricing_model(PricingModel {
            engine: "cohere".to_string(),
            model: "command".to_string(),
            prompt_rate: 0.001,     // $1 per 1M tokens (estimated)
            completion_rate: 0.002, // $2 per 1M tokens (estimated)
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // Mistral pricing (estimated)
        self.add_pricing_model(PricingModel {
            engine: "mistral".to_string(),
            model: "mistral-large".to_string(),
            prompt_rate: 0.004,     // $4 per 1M tokens (estimated)
            completion_rate: 0.012, // $12 per 1M tokens (estimated)
            image_rate: None,
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });

        // Replicate pricing (estimated for image generation)
        self.add_pricing_model(PricingModel {
            engine: "replicate".to_string(),
            model: "flux-pro".to_string(),
            prompt_rate: 0.0,       // No text tokens
            completion_rate: 0.0,   // No text tokens
            image_rate: Some(0.05), // $0.05 per image (estimated)
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        });
    }

    fn add_pricing_model(&mut self, model: PricingModel) {
        let key = format!("{}:{}", model.engine, model.model);
        self.pricing_models.insert(key, model);
    }

    /// Calculate cost with security validation
    pub fn calculate_cost(&mut self, engine: &str, model: &str, usage: &Usage) -> Result<Cost> {
        debug!(
            "Calculating cost for engine: {}, model: {}, usage: {:?}",
            engine, model, usage
        );

        // Find pricing model
        let pricing = self.find_pricing_model(engine, model)?;

        // Calculate costs (rates are per 1M tokens, so divide by 1,000,000)
        let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * pricing.prompt_rate;
        let completion_cost =
            (usage.completion_tokens as f64 / 1_000_000.0) * pricing.completion_rate;
        let mut total_cost = prompt_cost + completion_cost;

        // Add image cost if applicable
        if let Some(image_rate) = pricing.image_rate {
            // For image generation, assume 1 image per request
            total_cost += image_rate;
        }

        let cost = Cost {
            prompt_cost,
            completion_cost,
            total_cost,
        };

        // Security validation
        self.validate_cost(&cost)?;

        // Update daily total
        self.daily_total += total_cost;

        // Log for audit trail
        debug!(
            "Cost calculated: prompt=${:.6}, completion=${:.6}, total=${:.6}",
            prompt_cost, completion_cost, total_cost
        );

        if total_cost > self.limits.warn_threshold {
            warn!(
                "High cost detected: ${:.6} for engine: {}, model: {}",
                total_cost, engine, model
            );
        }

        Ok(cost)
    }

    fn find_pricing_model(&self, engine: &str, model: &str) -> Result<&PricingModel> {
        // Try exact match first
        let exact_key = format!("{}:{}", engine, model);
        if let Some(pricing) = self.pricing_models.get(&exact_key) {
            return Ok(pricing);
        }

        // Try partial matches for model variants
        for (key, pricing) in &self.pricing_models {
            if key.starts_with(&format!("{}:", engine)) && model.contains(&pricing.model) {
                debug!(
                    "Using partial match for pricing: {} -> {}",
                    model, pricing.model
                );
                return Ok(pricing);
            }
        }

        Err(anyhow!(
            "No pricing model found for engine: {}, model: {}",
            engine,
            model
        ))
    }

    fn validate_cost(&self, cost: &Cost) -> Result<()> {
        // Check for negative costs
        if cost.prompt_cost < 0.0 || cost.completion_cost < 0.0 || cost.total_cost < 0.0 {
            return Err(anyhow!("Invalid negative cost detected"));
        }

        // Check for unreasonably high costs
        if cost.total_cost > self.limits.max_single_request {
            return Err(anyhow!(
                "Cost ${:.6} exceeds maximum single request limit of ${:.2}",
                cost.total_cost,
                self.limits.max_single_request
            ));
        }

        // Check daily limit
        if self.daily_total + cost.total_cost > self.limits.max_daily_total {
            return Err(anyhow!(
                "Cost ${:.6} would exceed daily limit of ${:.2} (current: ${:.2})",
                cost.total_cost,
                self.limits.max_daily_total,
                self.daily_total
            ));
        }

        // Check for mathematical consistency
        let calculated_total = cost.prompt_cost + cost.completion_cost;
        if (cost.total_cost - calculated_total).abs() > 0.000001 {
            return Err(anyhow!("Cost calculation inconsistency detected"));
        }

        Ok(())
    }

    pub fn get_daily_total(&self) -> f64 {
        self.daily_total
    }

    pub fn reset_daily_total(&mut self) {
        self.daily_total = 0.0;
        debug!("Daily cost total reset");
    }
}

impl Default for CostCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Usage;

    #[test]
    fn test_cost_calculator_creation() {
        let calculator = CostCalculator::new();
        assert_eq!(calculator.get_daily_total(), 0.0);
    }

    #[test]
    fn test_calculate_openai_cost() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 1000000,    // 1M tokens
            completion_tokens: 500000, // 0.5M tokens
            total_tokens: 1500000,
        };

        let cost = calculator
            .calculate_cost("openai", "gpt-3.5-turbo", &usage)
            .unwrap();

        // GPT-3.5-turbo pricing: $0.0015/1M prompt, $0.002/1M completion
        assert_eq!(cost.prompt_cost, 0.0015);
        assert_eq!(cost.completion_cost, 0.001);
        assert_eq!(cost.total_cost, 0.0025);
    }

    #[test]
    fn test_calculate_anthropic_cost() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 1000000,    // 1M tokens
            completion_tokens: 500000, // 0.5M tokens
            total_tokens: 1500000,
        };

        let cost = calculator
            .calculate_cost("anthropic", "claude-3-sonnet", &usage)
            .unwrap();

        // Claude-3-sonnet pricing: $0.003/1M prompt, $0.015/1M completion
        assert_eq!(cost.prompt_cost, 0.003);
        assert_eq!(cost.completion_cost, 0.0075);
        // Use approximate comparison for floating point
        assert!((cost.total_cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_unknown_model_cost() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 1000000,
            completion_tokens: 500000,
            total_tokens: 1500000,
        };

        // Should return error for unknown model
        let result = calculator.calculate_cost("openai", "unknown-model", &usage);
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_tokens() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        };

        let cost = calculator
            .calculate_cost("openai", "gpt-4", &usage)
            .unwrap();

        assert_eq!(cost.prompt_cost, 0.0);
        assert_eq!(cost.completion_cost, 0.0);
        assert_eq!(cost.total_cost, 0.0);
    }

    #[test]
    fn test_daily_total_tracking() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 100000,    // 0.1M tokens
            completion_tokens: 50000, // 0.05M tokens
            total_tokens: 150000,
        };

        // Calculate cost and verify daily total is updated
        let initial_daily = calculator.get_daily_total();
        let cost = calculator
            .calculate_cost("openai", "gpt-3.5-turbo", &usage)
            .unwrap();
        let new_daily = calculator.get_daily_total();

        assert_eq!(new_daily, initial_daily + cost.total_cost);
    }

    #[test]
    fn test_reset_daily_total() {
        let mut calculator = CostCalculator::new();

        let usage = Usage {
            prompt_tokens: 100000,
            completion_tokens: 50000,
            total_tokens: 150000,
        };

        calculator
            .calculate_cost("openai", "gpt-3.5-turbo", &usage)
            .unwrap();
        assert!(calculator.get_daily_total() > 0.0);

        calculator.reset_daily_total();
        assert_eq!(calculator.get_daily_total(), 0.0);
    }
}
