use fluent_core::types::{Cost, ExtractedContent, Response, Usage};
use serde_json::Value;

/// Shared response parsing utilities for engines
pub struct ResponseParser;

impl ResponseParser {
    /// Parse OpenAI-style chat completion response
    pub fn parse_openai_chat_response(
        response: &Value,
        model: &str,
        pricing: Option<(f64, f64)>
    ) -> anyhow::Result<Response> {
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract content from response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        let cost = if let Some((prompt_price, completion_price)) = pricing {
            Cost {
                prompt_cost: (usage.prompt_tokens as f64 / 1000.0) * prompt_price,
                completion_cost: (usage.completion_tokens as f64 / 1000.0) * completion_price,
                total_cost: 0.0, // Will be calculated
            }
        } else {
            Cost {
                prompt_cost: 0.0,
                completion_cost: 0.0,
                total_cost: 0.0,
            }
        };

        let mut final_cost = cost;
        final_cost.total_cost = final_cost.prompt_cost + final_cost.completion_cost;

        let finish_reason = response["choices"][0]["finish_reason"]
            .as_str()
            .map(String::from);

        Ok(Response {
            content,
            usage,
            model: model.to_string(),
            finish_reason,
            cost: final_cost,
        })
    }

    /// Parse Anthropic-style response
    pub fn parse_anthropic_response(
        response: &Value,
        model: &str,
        pricing: Option<(f64, f64)>
    ) -> anyhow::Result<Response> {
        let content = response["content"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract content from Anthropic response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: response["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: 0, // Will be calculated
        };

        let mut final_usage = usage;
        final_usage.total_tokens = final_usage.prompt_tokens + final_usage.completion_tokens;

        let cost = if let Some((prompt_price, completion_price)) = pricing {
            Cost {
                prompt_cost: (final_usage.prompt_tokens as f64 / 1000.0) * prompt_price,
                completion_cost: (final_usage.completion_tokens as f64 / 1000.0) * completion_price,
                total_cost: 0.0, // Will be calculated
            }
        } else {
            Cost {
                prompt_cost: 0.0,
                completion_cost: 0.0,
                total_cost: 0.0,
            }
        };

        let mut final_cost = cost;
        final_cost.total_cost = final_cost.prompt_cost + final_cost.completion_cost;

        let finish_reason = response["stop_reason"]
            .as_str()
            .map(String::from);

        Ok(Response {
            content,
            usage: final_usage,
            model: model.to_string(),
            finish_reason,
            cost: final_cost,
        })
    }

    /// Parse Google Gemini response
    pub fn parse_gemini_response(
        response: &Value,
        model: &str,
        pricing: Option<(f64, f64)>
    ) -> anyhow::Result<Response> {
        let content = response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract content from Gemini response"))?
            .to_string();

        let usage = Usage {
            prompt_tokens: response["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
            total_tokens: response["usageMetadata"]["totalTokenCount"].as_u64().unwrap_or(0) as u32,
        };

        let cost = if let Some((prompt_price, completion_price)) = pricing {
            Cost {
                prompt_cost: (usage.prompt_tokens as f64 / 1000.0) * prompt_price,
                completion_cost: (usage.completion_tokens as f64 / 1000.0) * completion_price,
                total_cost: 0.0, // Will be calculated
            }
        } else {
            Cost {
                prompt_cost: 0.0,
                completion_cost: 0.0,
                total_cost: 0.0,
            }
        };

        let mut final_cost = cost;
        final_cost.total_cost = final_cost.prompt_cost + final_cost.completion_cost;

        let finish_reason = response["candidates"][0]["finishReason"]
            .as_str()
            .map(String::from);

        Ok(Response {
            content,
            usage,
            model: model.to_string(),
            finish_reason,
            cost: final_cost,
        })
    }

    /// Parse simple text response (for image generation, etc.)
    pub fn parse_simple_response(
        content: String,
        model: &str,
        estimated_tokens: Option<u32>
    ) -> Response {
        let tokens = estimated_tokens.unwrap_or(0);
        
        Response {
            content,
            usage: Usage {
                prompt_tokens: tokens,
                completion_tokens: 0,
                total_tokens: tokens,
            },
            model: model.to_string(),
            finish_reason: Some("complete".to_string()),
            cost: Cost {
                prompt_cost: 0.0,
                completion_cost: 0.0,
                total_cost: 0.0,
            },
        }
    }

    /// Extract content for different response formats
    pub fn extract_content_openai(value: &Value) -> Option<ExtractedContent> {
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    /// Extract content for Anthropic responses
    pub fn extract_content_anthropic(value: &Value) -> Option<ExtractedContent> {
        value["content"][0]["text"]
            .as_str()
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    /// Extract content for Gemini responses
    pub fn extract_content_gemini(value: &Value) -> Option<ExtractedContent> {
        value["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|content| ExtractedContent {
                main_content: content.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            })
    }

    /// Generic content extraction with fallback
    pub fn extract_content_generic(value: &Value) -> Option<ExtractedContent> {
        // Try different common response formats
        if let Some(content) = Self::extract_content_openai(value) {
            return Some(content);
        }
        
        if let Some(content) = Self::extract_content_anthropic(value) {
            return Some(content);
        }
        
        if let Some(content) = Self::extract_content_gemini(value) {
            return Some(content);
        }

        // Fallback to simple text extraction
        if let Some(text) = value.get("text").and_then(|v| v.as_str()) {
            return Some(ExtractedContent {
                main_content: text.to_string(),
                sentiment: None,
                clusters: None,
                themes: None,
                keywords: None,
            });
        }

        // Last resort: convert entire response to string
        Some(ExtractedContent {
            main_content: serde_json::to_string(value).ok()?,
            sentiment: None,
            clusters: None,
            themes: None,
            keywords: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_openai_chat_response() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Hello, world!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });

        let result = ResponseParser::parse_openai_chat_response(
            &response, 
            "gpt-3.5-turbo", 
            Some((0.001, 0.002))
        ).unwrap();

        assert_eq!(result.content, "Hello, world!");
        assert_eq!(result.usage.prompt_tokens, 10);
        assert_eq!(result.usage.completion_tokens, 5);
        assert_eq!(result.usage.total_tokens, 15);
        assert_eq!(result.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_extract_content_openai() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Test content"
                }
            }]
        });

        let content = ResponseParser::extract_content_openai(&response).unwrap();
        assert_eq!(content.main_content, "Test content");
    }

    #[test]
    fn test_parse_simple_response() {
        let response = ResponseParser::parse_simple_response(
            "Simple response".to_string(),
            "test-model",
            Some(20)
        );

        assert_eq!(response.content, "Simple response");
        assert_eq!(response.model, "test-model");
        assert_eq!(response.usage.total_tokens, 20);
        assert_eq!(response.finish_reason, Some("complete".to_string()));
    }
}
