//! Example demonstrating rate limiting with engines
//!
//! This example shows how to integrate the RateLimiter with engine requests
//! to prevent API throttling.
//!
//! Run with: cargo run --example rate_limiter_demo

use fluent_engines::RateLimiter;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("=== Rate Limiter Demo ===\n");

    // Example 1: Basic rate limiting
    println!("Example 1: Basic rate limiting (5 requests/second)");
    let limiter = RateLimiter::new(5.0);

    let start = Instant::now();
    for i in 1..=10 {
        limiter.acquire().await;
        println!("  Request {}: {:?} elapsed", i, start.elapsed());
    }
    println!("  Total time: {:?}\n", start.elapsed());

    // Example 2: Try acquire (non-blocking)
    println!("Example 2: Non-blocking try_acquire");
    let limiter = RateLimiter::new(2.0);

    for i in 1..=5 {
        if limiter.try_acquire().await {
            println!("  Request {}: Acquired token", i);
        } else {
            println!("  Request {}: No tokens available", i);
        }
    }
    println!();

    // Example 3: Monitoring available tokens
    println!("Example 3: Monitoring available tokens");
    let limiter = RateLimiter::new(10.0);

    println!("  Initial tokens: {:.2}", limiter.available_tokens().await);

    for _ in 0..5 {
        limiter.acquire().await;
    }

    println!("  After 5 requests: {:.2}", limiter.available_tokens().await);

    // Wait for refill
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    println!("  After 500ms wait: {:.2}", limiter.available_tokens().await);
    println!();

    // Example 4: Slow rate (1 request every 2 seconds)
    println!("Example 4: Slow rate (0.5 requests/second = 1 every 2 seconds)");
    let limiter = RateLimiter::new(0.5);

    let start = Instant::now();
    for i in 1..=3 {
        limiter.acquire().await;
        println!("  Request {}: {:?} elapsed", i, start.elapsed());
    }
    println!("  Total time: {:?}\n", start.elapsed());

    // Example 5: Simulating API calls with rate limiting
    println!("Example 5: Simulated API calls with rate limiting");
    simulate_api_calls().await;
}

async fn simulate_api_calls() {
    // Create a rate limiter for 3 requests per second
    let rate_limiter = RateLimiter::new(3.0);

    println!("  Making 10 'API calls' at 3 requests/second...");
    let start = Instant::now();

    for i in 1..=10 {
        // Wait for rate limit
        rate_limiter.acquire().await;

        // Simulate API call
        make_api_call(i).await;

        println!("    Call {} completed at {:?}", i, start.elapsed());
    }

    println!("  All calls completed in {:?}", start.elapsed());
}

async fn make_api_call(_call_number: i32) {
    // Simulate some API processing time
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    // In a real scenario, this would be an actual HTTP request to an LLM API
    // For example:
    // let response = client.post(url).json(&request).send().await?;
}
