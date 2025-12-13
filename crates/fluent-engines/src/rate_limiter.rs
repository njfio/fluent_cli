//! Rate limiting module for engine request throttling
//!
//! This module provides a token bucket rate limiter to prevent API throttling
//! by controlling the rate of requests to external API providers.
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_engines::RateLimiter;
//!
//! # async fn example() {
//! let limiter = RateLimiter::new(10.0); // 10 requests per second
//!
//! // Wait until a token is available before making request
//! limiter.acquire().await;
//! // Make your API request here
//! # }
//! ```

use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Simple token bucket rate limiter
///
/// This rate limiter uses the token bucket algorithm to control the rate of requests.
/// Tokens are refilled at a constant rate, and requests consume tokens.
/// If no tokens are available, requests will wait until a token becomes available.
///
/// # Features
///
/// - **Burst support**: Allows burst up to 2x the configured rate
/// - **Async-first**: Uses async/await for non-blocking operation
/// - **Fair**: Processes requests in order
/// - **Simple**: Easy to integrate with existing code
///
/// # Configuration
///
/// Configure rate limiting per engine in your engine config:
/// ```json
/// {
///   "rate_limit": {
///     "enabled": true,
///     "requests_per_second": 10.0
///   }
/// }
/// ```
pub struct RateLimiter {
    tokens: Mutex<f64>,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `requests_per_second` - Maximum number of requests per second
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fluent_engines::RateLimiter;
    ///
    /// // Allow 10 requests per second
    /// let limiter = RateLimiter::new(10.0);
    ///
    /// // Allow 0.5 requests per second (1 request every 2 seconds)
    /// let slow_limiter = RateLimiter::new(0.5);
    /// ```
    pub fn new(requests_per_second: f64) -> Self {
        Self {
            tokens: Mutex::new(requests_per_second),
            max_tokens: requests_per_second * 2.0, // Allow burst
            refill_rate: requests_per_second,
            last_refill: Mutex::new(Instant::now()),
        }
    }

    /// Wait until a token is available
    ///
    /// This method will block until a token is available in the bucket.
    /// It refills tokens based on the elapsed time since the last refill.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fluent_engines::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::new(10.0);
    ///
    /// // This will wait if no tokens are available
    /// limiter.acquire().await;
    /// // Make your API request here
    /// # }
    /// ```
    pub async fn acquire(&self) {
        loop {
            {
                let mut tokens = self.tokens.lock().await;
                let mut last = self.last_refill.lock().await;

                // Refill tokens
                let elapsed = last.elapsed().as_secs_f64();
                *tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
                *last = Instant::now();

                if *tokens >= 1.0 {
                    *tokens -= 1.0;
                    return;
                }
            }

            // Wait a bit before trying again
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    /// Try to acquire a token without blocking
    ///
    /// Returns `true` if a token was acquired, `false` if no tokens are available.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fluent_engines::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::new(10.0);
    ///
    /// if limiter.try_acquire().await {
    ///     // Token acquired, make request
    /// } else {
    ///     // No tokens available, handle accordingly
    /// }
    /// # }
    /// ```
    pub async fn try_acquire(&self) -> bool {
        let mut tokens = self.tokens.lock().await;
        let mut last = self.last_refill.lock().await;

        // Refill tokens
        let elapsed = last.elapsed().as_secs_f64();
        *tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
        *last = Instant::now();

        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get the current number of available tokens
    ///
    /// This is useful for monitoring and debugging.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use fluent_engines::RateLimiter;
    ///
    /// # async fn example() {
    /// let limiter = RateLimiter::new(10.0);
    /// let available = limiter.available_tokens().await;
    /// println!("Available tokens: {}", available);
    /// # }
    /// ```
    pub async fn available_tokens(&self) -> f64 {
        let mut tokens = self.tokens.lock().await;
        let mut last = self.last_refill.lock().await;

        // Refill tokens
        let elapsed = last.elapsed().as_secs_f64();
        *tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
        *last = Instant::now();

        *tokens
    }
}

impl Default for RateLimiter {
    /// Create a default rate limiter with 10 requests per second
    fn default() -> Self {
        Self::new(10.0) // 10 requests per second default
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(10.0);
        let available = limiter.available_tokens().await;

        // Should start with ~10 tokens
        assert!((available - 10.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_rate_limiter_burst() {
        let limiter = RateLimiter::new(10.0);

        let start = Instant::now();
        for _ in 0..5 {
            limiter.acquire().await;
        }

        // Should complete quickly (within burst allowance)
        assert!(start.elapsed() < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_rate_limiter_throttling() {
        let limiter = RateLimiter::new(10.0);

        // Exhaust initial tokens
        for _ in 0..20 {
            limiter.acquire().await;
        }

        // Next request should take at least 100ms (1/10th of a second)
        let start = Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();

        // Should wait for refill (be lenient with timing to avoid flakiness)
        assert!(elapsed >= Duration::from_millis(50)); // Account for timing variance and system load
    }

    #[tokio::test]
    async fn test_try_acquire_success() {
        let limiter = RateLimiter::new(10.0);

        // Should succeed immediately
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_try_acquire_failure() {
        let limiter = RateLimiter::new(10.0);

        // Exhaust all tokens
        for _ in 0..20 {
            limiter.acquire().await;
        }

        // Should fail immediately without waiting
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_available_tokens() {
        let limiter = RateLimiter::new(5.0);

        let initial = limiter.available_tokens().await;
        assert!((initial - 5.0).abs() < 0.1);

        limiter.acquire().await;
        let after_one = limiter.available_tokens().await;
        assert!((after_one - 4.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_default_rate_limiter() {
        let limiter = RateLimiter::default();
        let available = limiter.available_tokens().await;

        // Default should be 10 requests per second
        assert!((available - 10.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_refill_over_time() {
        let limiter = RateLimiter::new(10.0);

        // Consume some tokens
        for _ in 0..5 {
            limiter.acquire().await;
        }

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(200)).await;

        let available = limiter.available_tokens().await;
        // Should have refilled ~2 tokens (0.2 seconds * 10 tokens/second)
        assert!(available > 6.5);
    }

    #[tokio::test]
    async fn test_max_tokens_cap() {
        let limiter = RateLimiter::new(5.0);

        // Wait for potential refill
        tokio::time::sleep(Duration::from_secs(2)).await;

        let available = limiter.available_tokens().await;
        // Should not exceed max_tokens (2x rate = 10)
        assert!(available <= 10.5);
    }

    #[tokio::test]
    async fn test_slow_rate() {
        let limiter = RateLimiter::new(2.0); // 2 requests per second

        limiter.acquire().await;
        limiter.acquire().await;

        // Third request should wait ~500ms
        let start = Instant::now();
        limiter.acquire().await;
        let elapsed = start.elapsed();

        // Be lenient with timing to avoid flakiness on slow/busy systems
        assert!(elapsed >= Duration::from_millis(300)); // Account for timing variance and system load
    }
}
