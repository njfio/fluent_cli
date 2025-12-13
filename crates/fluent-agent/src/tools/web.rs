//! Web browsing and search tool executor
//!
//! Provides tools for fetching web content and performing web searches
//! using DuckDuckGo's HTML interface (no API key required).

use super::ToolExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Configuration for web tools
#[derive(Debug, Clone)]
pub struct WebConfig {
    /// Timeout for HTTP requests in seconds
    pub timeout_seconds: u64,
    /// Maximum response body size in bytes
    pub max_response_size: usize,
    /// User agent string for requests
    pub user_agent: String,
    /// Allowed domains (empty = all allowed)
    pub allowed_domains: Vec<String>,
    /// Blocked domains
    pub blocked_domains: Vec<String>,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_response_size: 512 * 1024, // 512KB
            user_agent:
                "Mozilla/5.0 (compatible; FluentAgent/1.0; +https://github.com/njfio/fluent_cli)"
                    .to_string(),
            allowed_domains: vec![],
            blocked_domains: vec![],
        }
    }
}

/// Result from fetching a URL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub url: String,
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content: String,
    pub truncated: bool,
    pub fetch_time_ms: u64,
}

/// Result from a web search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub results: Vec<SearchResultItem>,
    pub search_time_ms: u64,
}

/// A single search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Web tool executor for fetching URLs and searching the web
pub struct WebExecutor {
    config: WebConfig,
    client: reqwest::Client,
}

impl WebExecutor {
    /// Create a new web executor with the given configuration
    pub fn new(config: WebConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { config, client }
    }

    /// Create a web executor with default configuration
    pub fn with_defaults() -> Self {
        Self::new(WebConfig::default())
    }

    /// Check if a URL is allowed based on domain configuration
    fn is_url_allowed(&self, url: &str) -> Result<()> {
        let parsed = url::Url::parse(url).map_err(|e| anyhow!("Invalid URL: {}", e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| anyhow!("URL has no host"))?;

        // Check blocked domains (with proper subdomain matching)
        for blocked in &self.config.blocked_domains {
            // Match exact domain or subdomain (e.g., "sub.blocked.com" matches "blocked.com")
            if host == blocked.as_str() || host.ends_with(&format!(".{}", blocked)) {
                return Err(anyhow!("Domain '{}' is blocked", host));
            }
        }

        // If allowed domains specified, check against them
        if !self.config.allowed_domains.is_empty() {
            let allowed = self
                .config
                .allowed_domains
                .iter()
                .any(|d| host == d.as_str() || host.ends_with(&format!(".{}", d)));
            if !allowed {
                return Err(anyhow!("Domain '{}' is not in allowed list", host));
            }
        }

        Ok(())
    }

    /// Fetch content from a URL
    async fn fetch_url(&self, url: &str) -> Result<FetchResult> {
        self.is_url_allowed(url)?;

        let start = std::time::Instant::now();

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("HTTP request failed: {}", e))?;

        let status_code = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Read body with size limit
        let body = response
            .bytes()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))?;

        let truncated = body.len() > self.config.max_response_size;
        let body_slice = if truncated {
            &body[..self.config.max_response_size]
        } else {
            &body[..]
        };

        // Convert to string, handling potential encoding issues
        let content = String::from_utf8_lossy(body_slice).to_string();

        // Extract text content from HTML if it's an HTML response
        let processed_content = if content_type
            .as_ref()
            .map(|ct| ct.contains("text/html"))
            .unwrap_or(false)
        {
            extract_text_from_html(&content)
        } else {
            content
        };

        Ok(FetchResult {
            url: url.to_string(),
            status_code,
            content_type,
            content: processed_content,
            truncated,
            fetch_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Perform a web search using DuckDuckGo's HTML interface
    async fn web_search(&self, query: &str, max_results: usize) -> Result<SearchResult> {
        let start = std::time::Instant::now();

        // Use DuckDuckGo HTML search (no API key required)
        let search_url = format!(
            "https://html.duckduckgo.com/html/?q={}",
            urlencoding::encode(query)
        );

        let response = self
            .client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| anyhow!("Search request failed: {}", e))?;

        let body = response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read search results: {}", e))?;

        // Parse search results from HTML
        let results = parse_duckduckgo_results(&body, max_results);

        Ok(SearchResult {
            query: query.to_string(),
            results,
            search_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

/// Extract readable text from HTML content
fn extract_text_from_html(html: &str) -> String {
    // Remove script and style tags with their content
    let html = regex::Regex::new(r"(?is)<script[^>]*>.*?</script>")
        .map(|re| re.replace_all(html, "").to_string())
        .unwrap_or_else(|_| html.to_string());

    let html = regex::Regex::new(r"(?is)<style[^>]*>.*?</style>")
        .map(|re| re.replace_all(&html, "").to_string())
        .unwrap_or(html);

    // Remove HTML tags
    let text = regex::Regex::new(r"<[^>]+>")
        .map(|re| re.replace_all(&html, " ").to_string())
        .unwrap_or(html);

    // Decode common HTML entities
    let text = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");

    // Clean up whitespace
    let text = regex::Regex::new(r"\s+")
        .map(|re| re.replace_all(&text, " ").to_string())
        .unwrap_or(text);

    text.trim().to_string()
}

/// Parse DuckDuckGo HTML search results
fn parse_duckduckgo_results(html: &str, max_results: usize) -> Vec<SearchResultItem> {
    let mut results = Vec::new();

    // Match result blocks - DuckDuckGo uses class="result" for each result
    let result_re = regex::Regex::new(
        r#"(?is)<a[^>]*class="result__a"[^>]*href="([^"]*)"[^>]*>([^<]*)</a>.*?<a[^>]*class="result__snippet"[^>]*>([^<]*)</a>"#,
    );

    if let Ok(re) = result_re {
        for cap in re.captures_iter(html) {
            if results.len() >= max_results {
                break;
            }

            let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let title = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let snippet = cap.get(3).map(|m| m.as_str()).unwrap_or("");

            // Skip DuckDuckGo internal links
            if url.starts_with("//duckduckgo.com") || url.is_empty() {
                continue;
            }

            results.push(SearchResultItem {
                title: html_decode(title.trim()),
                url: url.to_string(),
                snippet: html_decode(snippet.trim()),
            });
        }
    }

    // Fallback: try simpler regex if the above didn't match
    if results.is_empty() {
        let simple_re =
            regex::Regex::new(r#"(?is)<a[^>]*href="(https?://[^"]+)"[^>]*>([^<]+)</a>"#);

        if let Ok(re) = simple_re {
            for cap in re.captures_iter(html) {
                if results.len() >= max_results {
                    break;
                }

                let url = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                let title = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                // Skip common non-result URLs
                if url.contains("duckduckgo.com") || url.contains("javascript:") || title.len() < 5
                {
                    continue;
                }

                results.push(SearchResultItem {
                    title: html_decode(title.trim()),
                    url: url.to_string(),
                    snippet: String::new(),
                });
            }
        }
    }

    results
}

/// Decode basic HTML entities in text
fn html_decode(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}

#[async_trait]
impl ToolExecutor for WebExecutor {
    async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        match tool_name {
            "fetch_url" | "web_fetch" => {
                let url = parameters
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing required parameter 'url'"))?;

                let result = self.fetch_url(url).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            "web_search" | "search" => {
                let query = parameters
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing required parameter 'query'"))?;

                let max_results = parameters
                    .get("max_results")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                let result = self.web_search(query, max_results).await?;
                Ok(serde_json::to_string_pretty(&result)?)
            }

            _ => Err(anyhow!("Unknown web tool: {}", tool_name)),
        }
    }

    fn get_available_tools(&self) -> Vec<String> {
        vec![
            "fetch_url".to_string(),
            "web_fetch".to_string(),
            "web_search".to_string(),
            "search".to_string(),
        ]
    }

    fn get_tool_description(&self, tool_name: &str) -> Option<String> {
        match tool_name {
            "fetch_url" | "web_fetch" => Some(
                "Fetch content from a URL. Parameters: url (required). Returns the page content as text.".to_string(),
            ),
            "web_search" | "search" => Some(
                "Search the web using DuckDuckGo. Parameters: query (required), max_results (optional, default 10). Returns search results with titles, URLs, and snippets.".to_string(),
            ),
            _ => None,
        }
    }

    fn validate_tool_request(
        &self,
        tool_name: &str,
        parameters: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        match tool_name {
            "fetch_url" | "web_fetch" => {
                let url = parameters
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing required parameter 'url'"))?;

                // Validate URL format and domain restrictions
                self.is_url_allowed(url)?;
                Ok(())
            }

            "web_search" | "search" => {
                if parameters.get("query").and_then(|v| v.as_str()).is_none() {
                    return Err(anyhow!("Missing required parameter 'query'"));
                }
                Ok(())
            }

            _ => Err(anyhow!("Unknown web tool: {}", tool_name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"
            <html>
            <head><script>console.log("test");</script><style>.foo {}</style></head>
            <body>
                <h1>Hello World</h1>
                <p>This is a &amp; test &lt;page&gt;</p>
            </body>
            </html>
        "#;

        let text = extract_text_from_html(html);
        assert!(text.contains("Hello World"));
        assert!(text.contains("This is a & test <page>"));
        assert!(!text.contains("console.log"));
        assert!(!text.contains("<script>"));
    }

    #[test]
    fn test_html_decode() {
        assert_eq!(html_decode("&amp;"), "&");
        assert_eq!(html_decode("&lt;foo&gt;"), "<foo>");
        assert_eq!(html_decode("&quot;test&quot;"), "\"test\"");
    }

    #[test]
    fn test_url_validation_blocked() {
        let config = WebConfig {
            blocked_domains: vec!["malware.com".to_string()],
            ..Default::default()
        };
        let executor = WebExecutor::new(config);

        assert!(executor.is_url_allowed("https://example.com").is_ok());
        assert!(executor.is_url_allowed("https://malware.com/page").is_err());
    }

    #[test]
    fn test_url_validation_allowed() {
        let config = WebConfig {
            allowed_domains: vec!["trusted.com".to_string()],
            ..Default::default()
        };
        let executor = WebExecutor::new(config);

        assert!(executor.is_url_allowed("https://trusted.com").is_ok());
        assert!(executor.is_url_allowed("https://untrusted.com").is_err());
    }

    #[test]
    fn test_get_available_tools() {
        let executor = WebExecutor::with_defaults();
        let tools = executor.get_available_tools();

        assert!(tools.contains(&"fetch_url".to_string()));
        assert!(tools.contains(&"web_search".to_string()));
    }
}
