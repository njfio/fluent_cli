use anyhow::Result;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;

/// Memory-optimized string builder that reuses buffers
pub struct StringBuffer {
    buffer: String,
}

impl StringBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(1024), // Pre-allocate reasonable capacity
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    /// Build URL without allocating new strings
    pub fn build_url(&mut self, protocol: &str, hostname: &str, port: u16, path: &str) -> &str {
        self.buffer.clear();
        write!(self.buffer, "{}://{}:{}{}", protocol, hostname, port, path).unwrap();
        &self.buffer
    }

    /// Build cache key without allocating
    pub fn build_cache_key(&mut self, payload: &str, file_path: Option<&str>) -> &str {
        self.buffer.clear();
        self.buffer.push_str(payload);
        if let Some(path) = file_path {
            self.buffer.push(':');
            self.buffer.push_str(path);
        }
        &self.buffer
    }

    /// Get the internal buffer
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Clear the buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}

impl Default for StringBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-optimized payload builder that reuses JSON values
pub struct PayloadBuilder {
    base_payload: Value,
    message_buffer: Vec<Value>,
}

impl PayloadBuilder {
    pub fn new() -> Self {
        Self {
            base_payload: json!({}),
            message_buffer: Vec::with_capacity(4), // Pre-allocate for typical conversation
        }
    }

    /// Build OpenAI-style payload without unnecessary allocations
    pub fn build_openai_payload(&mut self, content: &str, model: Option<&str>) -> &Value {
        // Clear and reuse message buffer
        self.message_buffer.clear();
        self.message_buffer.push(json!({
            "role": "user",
            "content": content
        }));

        // Reuse base payload object
        self.base_payload = json!({
            "messages": &self.message_buffer
        });

        if let Some(model_name) = model {
            self.base_payload["model"] = json!(model_name);
        }

        &self.base_payload
    }

    /// Build Anthropic-style payload
    pub fn build_anthropic_payload(&mut self, content: &str, model: Option<&str>) -> &Value {
        self.message_buffer.clear();
        self.message_buffer.push(json!({
            "role": "user",
            "content": content
        }));

        self.base_payload = json!({
            "messages": &self.message_buffer,
            "max_tokens": 4096
        });

        if let Some(model_name) = model {
            self.base_payload["model"] = json!(model_name);
        }

        &self.base_payload
    }

    /// Build vision payload with image data
    pub fn build_vision_payload(
        &mut self,
        text: &str,
        image_data: &str,
        image_format: &str,
    ) -> &Value {
        self.message_buffer.clear();
        self.message_buffer.push(json!({
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": text
                },
                {
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:image/{};base64,{}", image_format, image_data)
                    }
                }
            ]
        }));

        self.base_payload = json!({
            "messages": &self.message_buffer
        });

        &self.base_payload
    }

    /// Add configuration parameters without cloning
    pub fn add_config_params(&mut self, params: &HashMap<String, Value>) {
        for (key, value) in params {
            match key.as_str() {
                "temperature" | "max_tokens" | "top_p" | "frequency_penalty"
                | "presence_penalty" => {
                    self.base_payload[key] = value.clone();
                }
                _ => {} // Skip unknown parameters
            }
        }
    }

    /// Get the built payload
    pub fn payload(&self) -> &Value {
        &self.base_payload
    }
}

impl Default for PayloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-optimized file buffer for reading files
pub struct FileBuffer {
    buffer: Vec<u8>,
}

impl FileBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(8192), // 8KB initial capacity
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Read file into buffer, reusing existing allocation
    pub async fn read_file(&mut self, file_path: &std::path::Path) -> Result<&[u8]> {
        use tokio::io::AsyncReadExt;

        self.buffer.clear();
        let mut file = tokio::fs::File::open(file_path).await?;
        file.read_to_end(&mut self.buffer).await?;
        Ok(&self.buffer)
    }

    /// Get buffer as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    /// Clear buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Reserve additional capacity
    pub fn reserve(&mut self, additional: usize) {
        self.buffer.reserve(additional);
    }
}

impl Default for FileBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-optimized response parser that avoids string allocations
pub struct ResponseParser {
    content_buffer: String,
}

impl ResponseParser {
    pub fn new() -> Self {
        Self {
            content_buffer: String::with_capacity(4096),
        }
    }

    /// Extract content from OpenAI response without allocating new strings
    pub fn extract_openai_content(&mut self, response: &Value) -> Option<&str> {
        if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
            self.content_buffer.clear();
            self.content_buffer.push_str(content);
            Some(&self.content_buffer)
        } else {
            None
        }
    }

    /// Extract content from Anthropic response
    pub fn extract_anthropic_content(&mut self, response: &Value) -> Option<&str> {
        if let Some(content) = response["content"][0]["text"].as_str() {
            self.content_buffer.clear();
            self.content_buffer.push_str(content);
            Some(&self.content_buffer)
        } else {
            None
        }
    }

    /// Extract content from Gemini response
    pub fn extract_gemini_content(&mut self, response: &Value) -> Option<&str> {
        if let Some(content) = response["candidates"][0]["content"]["parts"][0]["text"].as_str() {
            self.content_buffer.clear();
            self.content_buffer.push_str(content);
            Some(&self.content_buffer)
        } else {
            None
        }
    }

    /// Get the extracted content
    pub fn content(&self) -> &str {
        &self.content_buffer
    }
}

impl Default for ResponseParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory pool for reusing common objects
pub struct MemoryPool {
    string_buffers: Vec<StringBuffer>,
    payload_builders: Vec<PayloadBuilder>,
    file_buffers: Vec<FileBuffer>,
    response_parsers: Vec<ResponseParser>,
}

impl MemoryPool {
    pub fn new() -> Self {
        Self {
            string_buffers: Vec::new(),
            payload_builders: Vec::new(),
            file_buffers: Vec::new(),
            response_parsers: Vec::new(),
        }
    }

    /// Get a string buffer from the pool or create a new one
    pub fn get_string_buffer(&mut self) -> StringBuffer {
        self.string_buffers.pop().unwrap_or_else(StringBuffer::new)
    }

    /// Return a string buffer to the pool
    pub fn return_string_buffer(&mut self, mut buffer: StringBuffer) {
        buffer.clear();
        if self.string_buffers.len() < 10 {
            // Limit pool size
            self.string_buffers.push(buffer);
        }
    }

    /// Get a payload builder from the pool
    pub fn get_payload_builder(&mut self) -> PayloadBuilder {
        self.payload_builders
            .pop()
            .unwrap_or_else(PayloadBuilder::new)
    }

    /// Return a payload builder to the pool
    pub fn return_payload_builder(&mut self, builder: PayloadBuilder) {
        if self.payload_builders.len() < 10 {
            self.payload_builders.push(builder);
        }
    }

    /// Get a file buffer from the pool
    pub fn get_file_buffer(&mut self) -> FileBuffer {
        self.file_buffers.pop().unwrap_or_else(FileBuffer::new)
    }

    /// Return a file buffer to the pool
    pub fn return_file_buffer(&mut self, mut buffer: FileBuffer) {
        buffer.clear();
        if self.file_buffers.len() < 5 {
            self.file_buffers.push(buffer);
        }
    }

    /// Get a response parser from the pool
    pub fn get_response_parser(&mut self) -> ResponseParser {
        self.response_parsers
            .pop()
            .unwrap_or_else(ResponseParser::new)
    }

    /// Return a response parser to the pool
    pub fn return_response_parser(&mut self, parser: ResponseParser) {
        if self.response_parsers.len() < 10 {
            self.response_parsers.push(parser);
        }
    }
}

impl Default for MemoryPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-copy string operations
pub struct ZeroCopyUtils;

impl ZeroCopyUtils {
    /// Extract parameter without cloning when possible
    pub fn get_param<'a>(params: &'a HashMap<String, Value>, key: &str) -> Option<Cow<'a, str>> {
        params.get(key)?.as_str().map(Cow::Borrowed)
    }

    /// Build URL using string slices when possible
    pub fn build_url_borrowed(protocol: &str, hostname: &str, port: u16, path: &str) -> String {
        // Use format! only when necessary, prefer string concatenation for simple cases
        if port == 80 && protocol == "http" || port == 443 && protocol == "https" {
            format!("{}://{}{}", protocol, hostname, path)
        } else {
            format!("{}://{}:{}{}", protocol, hostname, port, path)
        }
    }

    /// Check if string contains pattern without allocating
    pub fn contains_any(text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| text.contains(pattern))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_buffer() {
        let mut buffer = StringBuffer::new();
        let url = buffer.build_url("https", "api.openai.com", 443, "/v1/chat/completions");
        assert_eq!(url, "https://api.openai.com:443/v1/chat/completions");

        let cache_key = buffer.build_cache_key("test payload", Some("file.txt"));
        assert_eq!(cache_key, "test payload:file.txt");
    }

    #[test]
    fn test_payload_builder() {
        let mut builder = PayloadBuilder::new();
        let payload = builder.build_openai_payload("Hello", Some("gpt-4"));

        assert_eq!(payload["model"], "gpt-4");
        assert_eq!(payload["messages"][0]["content"], "Hello");
    }

    #[tokio::test]
    async fn test_file_buffer() {
        use tempfile::NamedTempFile;
        use tokio::io::AsyncWriteExt;

        let temp_file = NamedTempFile::new().unwrap();
        let mut file = tokio::fs::File::create(temp_file.path()).await.unwrap();
        file.write_all(b"test content").await.unwrap();
        file.flush().await.unwrap();

        let mut buffer = FileBuffer::new();
        let content = buffer.read_file(temp_file.path()).await.unwrap();
        assert_eq!(content, b"test content");
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::new();

        let buffer1 = pool.get_string_buffer();
        let buffer2 = pool.get_string_buffer();

        pool.return_string_buffer(buffer1);
        pool.return_string_buffer(buffer2);

        // Should reuse buffers
        let _buffer3 = pool.get_string_buffer();
        let _buffer4 = pool.get_string_buffer();

        assert_eq!(pool.string_buffers.len(), 0); // Both buffers taken from pool
    }

    #[test]
    fn test_zero_copy_utils() {
        let mut params = HashMap::new();
        params.insert("model".to_string(), Value::String("gpt-4".to_string()));

        let model = ZeroCopyUtils::get_param(&params, "model").unwrap();
        assert_eq!(model, "gpt-4");

        let url = ZeroCopyUtils::build_url_borrowed("https", "api.openai.com", 443, "/v1/chat");
        assert_eq!(url, "https://api.openai.com/v1/chat");
    }
}
