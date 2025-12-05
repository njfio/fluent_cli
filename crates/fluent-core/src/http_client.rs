//! Secure HTTP client configuration with hardened defaults
//!
//! This module provides centralized HTTP client creation with:
//! - rustls-tls for secure TLS connections
//! - Sensible timeouts for connect and request operations
//! - Connection pooling and keepalive settings
//! - Proxy support via environment variables
//!
//! # Examples
//!
//! ```rust,no_run
//! use fluent_core::http_client::create_secure_client;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let client = create_secure_client()?;
//! let response = client.get("https://api.example.com").send().await?;
//! # Ok(())
//! # }
//! ```

use anyhow::{anyhow, Result};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use tracing::debug; // Using log instead of tracing for compatibility

/// Default timeout for establishing HTTP connections (10 seconds)
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default timeout for complete HTTP requests (30 seconds)
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum idle connections to keep per host
pub const DEFAULT_POOL_MAX_IDLE: usize = 10;

/// How long to keep idle connections alive
pub const DEFAULT_POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// TCP keepalive interval
pub const DEFAULT_TCP_KEEPALIVE: Duration = Duration::from_secs(60);

/// Create a secure HTTP client with sensible defaults
///
/// This function creates a reqwest HTTP client configured with:
/// - **rustls-tls**: Secure TLS implementation without relying on system OpenSSL
/// - **Connect timeout**: 10 seconds to establish connection
/// - **Request timeout**: 30 seconds for complete request/response
/// - **Connection pooling**: Up to 10 idle connections per host
/// - **TCP keepalive**: 60 second intervals
/// - **Proxy support**: Respects HTTP_PROXY, HTTPS_PROXY environment variables
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be built (rare, usually indicates
/// system resource exhaustion or invalid proxy configuration).
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::http_client::create_secure_client;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = create_secure_client()?;
/// let resp = client.get("https://api.openai.com/v1/models").send().await?;
/// println!("Status: {}", resp.status());
/// # Ok(())
/// # }
/// ```
pub fn create_secure_client() -> Result<Client> {
    create_client_with_timeout(DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT)
}

/// Create an HTTP client with custom timeouts
///
/// Use this when you need different timeout settings than the defaults.
/// For example, some APIs (like Anthropic with long responses) may need
/// longer request timeouts.
///
/// # Arguments
///
/// * `connect_timeout` - Maximum time to establish a connection
/// * `request_timeout` - Maximum time for the entire request/response cycle
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be built.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::http_client::create_client_with_timeout;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Create client with extended timeouts for slow APIs
/// let client = create_client_with_timeout(
///     Duration::from_secs(30),   // 30s connect timeout
///     Duration::from_secs(600),  // 10min request timeout
/// )?;
/// # Ok(())
/// # }
/// ```
pub fn create_client_with_timeout(
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<Client> {
    let mut builder = Client::builder()
        .use_rustls_tls() // Explicitly use rustls instead of native TLS
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .pool_max_idle_per_host(DEFAULT_POOL_MAX_IDLE)
        .pool_idle_timeout(DEFAULT_POOL_IDLE_TIMEOUT)
        .tcp_keepalive(DEFAULT_TCP_KEEPALIVE);

    // Support proxy configuration via environment variables
    // Check HTTPS_PROXY first, then HTTP_PROXY
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
            debug!("Using HTTPS proxy from environment: {}", proxy_url);
        }
    } else if let Ok(proxy_url) =
        std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
            debug!("Using HTTP proxy from environment: {}", proxy_url);
        }
    }

    builder
        .build()
        .map_err(|e| anyhow!("Failed to create secure HTTP client: {}", e))
}

/// Create a client builder with secure defaults pre-configured
///
/// Use this when you need to further customize the client beyond timeouts,
/// such as adding custom headers or authentication. The builder comes
/// pre-configured with rustls-tls, timeouts, and connection pooling.
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::http_client::create_secure_client_builder;
/// use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
///
/// # async fn example() -> anyhow::Result<()> {
/// let mut headers = HeaderMap::new();
/// headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer token"));
///
/// let client = create_secure_client_builder()
///     .default_headers(headers)
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub fn create_secure_client_builder() -> ClientBuilder {
    create_client_builder_with_timeout(DEFAULT_CONNECT_TIMEOUT, DEFAULT_REQUEST_TIMEOUT)
}

/// Create a client builder with custom timeouts and secure defaults
///
/// This is the most flexible option - returns a ClientBuilder that you can
/// further customize before calling `.build()`.
///
/// # Arguments
///
/// * `connect_timeout` - Maximum time to establish a connection
/// * `request_timeout` - Maximum time for the entire request/response cycle
///
/// # Examples
///
/// ```rust,no_run
/// use fluent_core::http_client::create_client_builder_with_timeout;
/// use std::time::Duration;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = create_client_builder_with_timeout(
///     Duration::from_secs(15),
///     Duration::from_secs(120),
/// )
/// .user_agent("my-custom-agent/1.0")
/// .build()?;
/// # Ok(())
/// # }
/// ```
pub fn create_client_builder_with_timeout(
    connect_timeout: Duration,
    request_timeout: Duration,
) -> ClientBuilder {
    let mut builder = Client::builder()
        .use_rustls_tls()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout)
        .pool_max_idle_per_host(DEFAULT_POOL_MAX_IDLE)
        .pool_idle_timeout(DEFAULT_POOL_IDLE_TIMEOUT)
        .tcp_keepalive(DEFAULT_TCP_KEEPALIVE);

    // Support proxy configuration
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
            debug!("Using HTTPS proxy from environment: {}", proxy_url);
        }
    } else if let Ok(proxy_url) =
        std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
            debug!("Using HTTP proxy from environment: {}", proxy_url);
        }
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_secure_client() {
        let client = create_secure_client();
        assert!(client.is_ok(), "Should create client successfully");
    }

    #[test]
    fn test_create_client_with_custom_timeouts() {
        let client = create_client_with_timeout(Duration::from_secs(5), Duration::from_secs(15));
        assert!(client.is_ok(), "Should create client with custom timeouts");
    }

    #[test]
    fn test_create_secure_client_builder() {
        let builder = create_secure_client_builder();
        let client = builder.build();
        assert!(client.is_ok(), "Should build client from builder");
    }

    #[test]
    fn test_client_builder_customization() {
        let client = create_secure_client_builder()
            .user_agent("test-agent/1.0")
            .build();
        assert!(client.is_ok(), "Should build customized client");
    }
}
