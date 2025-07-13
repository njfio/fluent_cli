//! Request processing and handling
//!
//! This module handles the processing of user requests, including file uploads,
//! content preparation, and request execution.

use anyhow::Result;
use fluent_core::traits::Engine;
use fluent_core::types::{Request, Response};
use std::path::Path;
use std::pin::Pin;
use tokio::fs;

/// Process a request with an optional file upload
pub async fn process_request_with_file(
    engine: &dyn Engine,
    request_content: &str,
    file_path: Option<&str>,
) -> Result<Response> {
    let mut final_content = request_content.to_string();

    // Handle file upload if provided
    if let Some(file_path) = file_path {
        let file_url = Pin::from(engine.upload_file(Path::new(file_path))).await?;
        final_content = format!("{}\n\nFile uploaded: {}", final_content, file_url);
    }

    process_request(engine, &final_content).await
}

/// Process a basic request without file upload
pub async fn process_request(engine: &dyn Engine, request_content: &str) -> Result<Response> {
    let request = Request {
        flowname: "user_request".to_string(),
        payload: request_content.to_string(),
    };

    Pin::from(engine.execute(&request)).await
}

/// Read and validate a file for processing
pub async fn read_file_content(file_path: &str) -> Result<String> {
    if !Path::new(file_path).exists() {
        return Err(anyhow::anyhow!("File does not exist: {}", file_path));
    }

    let content = fs::read_to_string(file_path).await?;
    
    if content.is_empty() {
        return Err(anyhow::anyhow!("File is empty: {}", file_path));
    }

    Ok(content)
}

/// Validate file size and type for upload
pub async fn validate_file_for_upload(file_path: &str) -> Result<()> {
    let path = Path::new(file_path);

    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist: {}", file_path));
    }

    let metadata = tokio::fs::metadata(path).await?;
    let file_size = metadata.len();
    
    // Check file size (limit to 10MB)
    const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
    if file_size > MAX_FILE_SIZE {
        return Err(anyhow::anyhow!(
            "File too large: {} bytes (max: {} bytes)",
            file_size,
            MAX_FILE_SIZE
        ));
    }

    // Check file extension for allowed types
    if let Some(extension) = path.extension() {
        let ext = extension.to_string_lossy().to_lowercase();
        let allowed_extensions = vec![
            "txt", "md", "json", "yaml", "yml", "toml", "csv",
            "py", "rs", "js", "ts", "html", "css", "xml",
            "png", "jpg", "jpeg", "gif", "pdf", "doc", "docx"
        ];
        
        if !allowed_extensions.contains(&ext.as_str()) {
            return Err(anyhow::anyhow!(
                "Unsupported file type: .{}",
                ext
            ));
        }
    }

    Ok(())
}

/// Prepare request content with context and formatting
pub fn prepare_request_content(
    base_content: &str,
    context: Option<&str>,
    format_instructions: Option<&str>,
) -> String {
    let mut content = base_content.to_string();

    if let Some(ctx) = context {
        content = format!("Context: {}\n\nRequest: {}", ctx, content);
    }

    if let Some(format_inst) = format_instructions {
        content = format!("{}\n\nFormat instructions: {}", content, format_inst);
    }

    content
}

/// Extract code blocks from content
pub fn extract_code_blocks(content: &str) -> Vec<(Option<String>, String)> {
    let mut code_blocks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        
        if line.starts_with("```") {
            let language = if line.len() > 3 {
                Some(line[3..].trim().to_string())
            } else {
                None
            };
            
            i += 1;
            let mut code_content = String::new();
            
            while i < lines.len() && !lines[i].trim().starts_with("```") {
                if !code_content.is_empty() {
                    code_content.push('\n');
                }
                code_content.push_str(lines[i]);
                i += 1;
            }
            
            if !code_content.trim().is_empty() {
                code_blocks.push((language, code_content));
            }
        }
        
        i += 1;
    }

    code_blocks
}

/// Sanitize content for safe processing
pub fn sanitize_content(content: &str) -> String {
    // Remove potentially dangerous content
    content
        .replace("<!--", "")
        .replace("-->", "")
        .replace("<script", "&lt;script")
        .replace("</script>", "&lt;/script&gt;")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_request_content() {
        let base = "Hello world";
        let context = "This is a test";
        let format = "Please respond in JSON";

        let result = prepare_request_content(base, Some(context), Some(format));
        assert!(result.contains("Context: This is a test"));
        assert!(result.contains("Request: Hello world"));
        assert!(result.contains("Format instructions: Please respond in JSON"));
    }

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"
Here's some code:

```rust
fn main() {
    println!("Hello, world!");
}
```

And some Python:

```python
print("Hello, world!")
```
"#;

        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, Some("rust".to_string()));
        assert!(blocks[0].1.contains("fn main()"));
        assert_eq!(blocks[1].0, Some("python".to_string()));
        assert!(blocks[1].1.contains("print("));
    }

    #[test]
    fn test_sanitize_content() {
        let dangerous = "Hello <!-- comment --> <script>alert('xss')</script>";
        let safe = sanitize_content(dangerous);
        assert!(!safe.contains("<!--"));
        assert!(!safe.contains("<script"));
        assert!(safe.contains("&lt;script"));
    }
}
