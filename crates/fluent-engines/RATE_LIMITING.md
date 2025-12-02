# Rate Limiting in Fluent Engines

This document describes the rate limiting functionality available in the fluent-engines crate.

## Overview

The rate limiter uses a **token bucket algorithm** to control the rate of API requests, preventing throttling by external providers like OpenAI, Anthropic, Google Gemini, etc.

## Features

- **Token Bucket Algorithm**: Efficient rate limiting with burst support
- **Async-First**: Non-blocking operation using Tokio
- **Configurable**: Per-engine rate limits via configuration
- **Flexible**: Supports fractional rates (e.g., 0.5 = 1 request every 2 seconds)
- **Burst Support**: Allows bursts up to 2x the configured rate
- **Monitoring**: Check available tokens at any time

## Usage

### Basic Usage

```rust
use fluent_engines::RateLimiter;

#[tokio::main]
async fn main() {
    // Create a rate limiter allowing 10 requests per second
    let limiter = RateLimiter::new(10.0);

    // Wait until a token is available (blocking)
    limiter.acquire().await;
    // Make your API request here
}
```

### Non-Blocking Acquire

```rust
use fluent_engines::RateLimiter;

#[tokio::main]
async fn main() {
    let limiter = RateLimiter::new(10.0);

    // Try to acquire without blocking
    if limiter.try_acquire().await {
        // Token acquired, proceed with request
    } else {
        // No tokens available, handle accordingly
    }
}
```

### Monitoring Available Tokens

```rust
use fluent_engines::RateLimiter;

#[tokio::main]
async fn main() {
    let limiter = RateLimiter::new(10.0);

    let available = limiter.available_tokens().await;
    println!("Available tokens: {}", available);
}
```

## Configuration

Rate limiting can be configured per engine in the `EnhancedEngineConfig`:

```json
{
  "name": "my-openai-engine",
  "engine": "openai",
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 10.0
  },
  "connection": {
    "protocol": "https",
    "hostname": "api.openai.com",
    "port": 443,
    "request_path": "/v1/chat/completions"
  },
  "parameters": {
    "model": "gpt-4",
    "temperature": 0.7
  }
}
```

### Configuration Options

- `enabled` (boolean): Enable or disable rate limiting for this engine
- `requests_per_second` (float): Maximum requests per second
  - Can be fractional (e.g., `0.5` = 1 request every 2 seconds)
  - Supports burst up to 2x this value

### Default Configuration

If not specified, the default configuration is:
- `enabled`: `false`
- `requests_per_second`: `10.0`

## Integration with Engines

To integrate rate limiting into an engine implementation:

```rust
use fluent_engines::RateLimiter;
use std::sync::Arc;

pub struct MyEngine {
    config: EngineConfig,
    client: reqwest::Client,
    rate_limiter: Option<Arc<RateLimiter>>,
}

impl MyEngine {
    pub async fn new(config: EnhancedEngineConfig) -> Result<Self> {
        // Create rate limiter if enabled
        let rate_limiter = if config.rate_limit.enabled {
            Some(Arc::new(RateLimiter::new(
                config.rate_limit.requests_per_second
            )))
        } else {
            None
        };

        Ok(Self {
            config: config.base,
            client: reqwest::Client::new(),
            rate_limiter,
        })
    }
}

impl Engine for MyEngine {
    async fn execute(&self, request: &Request) -> Result<Response> {
        // Apply rate limiting if enabled
        if let Some(limiter) = &self.rate_limiter {
            limiter.acquire().await;
        }

        // Make the actual API request
        let response = self.client.post("https://api.example.com")
            .json(&request)
            .send()
            .await?;

        // Process response...
        Ok(response)
    }
}
```

## Examples

### Example 1: Conservative Rate Limiting

For APIs with strict rate limits (e.g., free tier):

```json
{
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 0.5
  }
}
```

This allows **1 request every 2 seconds**.

### Example 2: Moderate Rate Limiting

For standard API usage:

```json
{
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 10.0
  }
}
```

This allows **10 requests per second** with burst support.

### Example 3: High-Volume Rate Limiting

For premium/enterprise tiers:

```json
{
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 100.0
  }
}
```

This allows **100 requests per second** with burst up to 200.

### Example 4: Disabled Rate Limiting

For local or unlimited APIs:

```json
{
  "rate_limit": {
    "enabled": false
  }
}
```

## Algorithm Details

The rate limiter uses a **token bucket** algorithm:

1. **Bucket Capacity**: `max_tokens = requests_per_second * 2.0`
   - This allows for burst traffic up to 2x the configured rate

2. **Refill Rate**: `requests_per_second` tokens are added per second
   - Refill happens continuously based on elapsed time

3. **Token Consumption**: Each request consumes 1 token
   - If no tokens are available, the request waits

4. **Overflow Protection**: Tokens are capped at `max_tokens`
   - Prevents infinite accumulation during idle periods

## Performance Characteristics

- **Minimal Overhead**: Token bucket operations are O(1)
- **Memory Efficient**: Only stores 4 values per limiter
- **Lock Contention**: Uses Tokio Mutex for async-friendly locking
- **Fair Scheduling**: Processes requests in order (FIFO)

## Testing

The rate limiter includes comprehensive tests:

```bash
# Run all rate limiter tests
cargo test -p fluent-engines rate_limiter -- --nocapture

# Run demo example
cargo run --example rate_limiter_demo
```

## Common Rate Limits by Provider

Reference values for common LLM providers (as of 2024):

| Provider | Free Tier | Paid Tier | Enterprise |
|----------|-----------|-----------|------------|
| OpenAI | 3 RPM | 60 RPM | Custom |
| Anthropic | 5 RPM | 50 RPM | Custom |
| Google Gemini | 60 RPM | 1000 RPM | Custom |
| Cohere | 100 RPM | 1000 RPM | Custom |
| Mistral | 10 RPM | 100 RPM | Custom |

*RPM = Requests Per Minute*

To configure for these limits:
- **3 RPM** = `0.05` requests per second
- **60 RPM** = `1.0` requests per second
- **1000 RPM** = `16.67` requests per second

## Troubleshooting

### Issue: Requests are too slow

**Solution**: Check your configured rate limit:
```rust
let available = limiter.available_tokens().await;
println!("Available tokens: {}", available);
```

If tokens are depleted, increase `requests_per_second` or wait for refill.

### Issue: Still getting rate limit errors from API

**Solution**: Your configured rate may be too high. Reduce `requests_per_second` to match your API tier's limits with some buffer:

```json
{
  "rate_limit": {
    "enabled": true,
    "requests_per_second": 0.9  // 90% of actual limit
  }
}
```

### Issue: Burst traffic not working

**Solution**: The rate limiter allows burst up to 2x the configured rate. Check if you're exhausting the burst allowance:

```rust
// Burst example
let limiter = RateLimiter::new(5.0); // 5 req/sec

// These 10 requests will complete quickly (burst)
for i in 1..=10 {
    limiter.acquire().await;
}

// But the next 10 will be throttled to 5 req/sec
```

## Future Enhancements

Potential future improvements:

- [ ] Exponential backoff integration
- [ ] Dynamic rate adjustment based on 429 responses
- [ ] Per-model rate limiting
- [ ] Rate limit sharing across multiple instances
- [ ] Metrics and monitoring integration
- [ ] Priority queuing for requests

## References

- [Token Bucket Algorithm (Wikipedia)](https://en.wikipedia.org/wiki/Token_bucket)
- [OpenAI Rate Limits](https://platform.openai.com/docs/guides/rate-limits)
- [Anthropic Rate Limits](https://docs.anthropic.com/claude/reference/rate-limits)
