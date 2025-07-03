use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashMap;

/// Memory-efficient string handling utilities
pub struct StringUtils;

impl StringUtils {
    /// Extract string value without cloning when possible
    pub fn extract_str<'a>(value: &'a Value, key: &str) -> Option<Cow<'a, str>> {
        value.get(key)?.as_str().map(Cow::Borrowed)
    }

    /// Extract string value with fallback
    pub fn extract_str_or<'a>(value: &'a Value, key: &str, default: &'a str) -> Cow<'a, str> {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(default))
    }

    /// Build string efficiently using references when possible
    pub fn build_url(protocol: &str, hostname: &str, port: u16, path: &str) -> String {
        let default_port = match protocol {
            "http" => 80,
            "https" => 443,
            _ => 0,
        };

        if port == default_port {
            format!("{}://{}{}", protocol, hostname, path)
        } else {
            format!("{}://{}:{}{}", protocol, hostname, port, path)
        }
    }

    /// Efficiently concatenate strings with minimal allocations
    pub fn concat_with_separator(parts: &[&str], separator: &str) -> String {
        if parts.is_empty() {
            return String::new();
        }

        let total_len =
            parts.iter().map(|s| s.len()).sum::<usize>() + separator.len() * (parts.len() - 1);

        let mut result = String::with_capacity(total_len);
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                result.push_str(separator);
            }
            result.push_str(part);
        }
        result
    }
}

/// Memory-efficient parameter handling
pub struct ParamUtils;

impl ParamUtils {
    /// Extract parameter without cloning when possible
    pub fn get_param<'a>(params: &'a HashMap<String, Value>, key: &str) -> Option<Cow<'a, str>> {
        params.get(key)?.as_str().map(Cow::Borrowed)
    }

    /// Extract parameter with type conversion
    pub fn get_param_as<T>(params: &HashMap<String, Value>, key: &str) -> Option<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let value = params.get(key)?;
        serde_json::from_value(value.clone()).ok()
    }

    /// Extract parameter with default value
    pub fn get_param_or<'a>(
        params: &'a HashMap<String, Value>,
        key: &str,
        default: &'a str,
    ) -> Cow<'a, str> {
        params
            .get(key)
            .and_then(|v| v.as_str())
            .map(Cow::Borrowed)
            .unwrap_or(Cow::Borrowed(default))
    }

    /// Build parameters map efficiently
    pub fn build_params(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        let mut params = HashMap::with_capacity(pairs.len());
        for (key, value) in pairs {
            params.insert(key.to_string(), Value::String(value.to_string()));
        }
        params
    }
}

/// Object pool for reusing allocations
pub struct ObjectPool<T> {
    objects: Vec<T>,
    max_size: usize,
}

impl<T> ObjectPool<T> {
    pub fn new(max_size: usize) -> Self {
        Self {
            objects: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Get an object from the pool or create a new one
    pub fn get<F>(&mut self, factory: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.objects.pop().unwrap_or_else(factory)
    }

    /// Return an object to the pool
    pub fn return_object(&mut self, obj: T) {
        if self.objects.len() < self.max_size {
            self.objects.push(obj);
        }
    }

    /// Get current pool size
    pub fn size(&self) -> usize {
        self.objects.len()
    }
}

/// Reusable string buffer
pub struct StringBuffer {
    buffer: String,
}

impl StringBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(1024),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
        }
    }

    /// Clear the buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get the buffer as a string slice
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Write to the buffer
    pub fn write_str(&mut self, s: &str) {
        self.buffer.push_str(s);
    }

    /// Write formatted content to the buffer
    pub fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        use std::fmt::Write;
        let _ = write!(self.buffer, "{}", args);
    }

    /// Take the string, leaving an empty buffer
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.buffer)
    }

    /// Get the current length
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for StringBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-efficient response processing
pub struct ResponseUtils;

impl ResponseUtils {
    /// Extract content without unnecessary cloning
    pub fn extract_content<'a>(response: &'a Value) -> Option<Cow<'a, str>> {
        // Try different common response formats
        if let Some(content) = response.get("content").and_then(|v| v.as_str()) {
            return Some(Cow::Borrowed(content));
        }

        if let Some(content) = response.get("text").and_then(|v| v.as_str()) {
            return Some(Cow::Borrowed(content));
        }

        if let Some(content) = response.get("message").and_then(|v| v.as_str()) {
            return Some(Cow::Borrowed(content));
        }

        // Try nested structures
        if let Some(content) = response
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|v| v.as_str())
        {
            return Some(Cow::Borrowed(content));
        }

        None
    }

    /// Extract error message efficiently
    pub fn extract_error<'a>(response: &'a Value) -> Option<Cow<'a, str>> {
        response
            .get("error")
            .and_then(|v| v.as_str())
            .map(Cow::Borrowed)
            .or_else(|| {
                response
                    .get("error")
                    .and_then(|v| v.get("message"))
                    .and_then(|v| v.as_str())
                    .map(Cow::Borrowed)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_string_utils() {
        let value = json!({"key": "value", "number": 42});

        let extracted = StringUtils::extract_str(&value, "key");
        assert_eq!(extracted, Some(Cow::Borrowed("value")));

        let with_default = StringUtils::extract_str_or(&value, "missing", "default");
        assert_eq!(with_default, Cow::Borrowed("default"));
    }

    #[test]
    fn test_object_pool() {
        let mut pool = ObjectPool::new(2);

        let obj1 = pool.get(|| String::from("test"));
        assert_eq!(obj1, "test");

        pool.return_object(String::from("reused"));
        let obj2 = pool.get(|| String::from("new"));
        assert_eq!(obj2, "reused");
    }

    #[test]
    fn test_string_buffer() {
        let mut buffer = StringBuffer::new();
        buffer.write_str("Hello");
        buffer.write_str(" World");

        assert_eq!(buffer.as_str(), "Hello World");
        assert_eq!(buffer.len(), 11);

        buffer.clear();
        assert!(buffer.is_empty());
    }
}
