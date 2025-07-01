use anyhow::{anyhow, Result};
use log::{debug, warn};
use regex::Regex;
use std::path::PathBuf;
use url::Url;
use uuid;

/// Input validation and sanitization utilities
pub struct InputValidator;

impl InputValidator {
    /// Validates and sanitizes a request payload
    pub fn validate_request_payload(payload: &str) -> Result<String> {
        // Check payload length
        if payload.is_empty() {
            return Err(anyhow!("Request payload cannot be empty"));
        }
        
        if payload.len() > 1_000_000 { // 1MB limit
            return Err(anyhow!("Request payload too large: {} bytes", payload.len()));
        }
        
        // Check for potential injection patterns
        Self::check_for_injection_patterns(payload)?;
        
        // Sanitize control characters but preserve newlines and tabs
        let sanitized = payload
            .chars()
            .filter(|c| {
                c.is_ascii_graphic() || 
                c.is_ascii_whitespace() || 
                *c == '\n' || 
                *c == '\t' || 
                *c == '\r' ||
                !c.is_ascii() // Allow non-ASCII characters (Unicode)
            })
            .collect();
            
        debug!("Validated request payload: {} chars", payload.len());
        Ok(sanitized)
    }
    
    /// Validates URL components for safety
    pub fn validate_url_components(protocol: &str, hostname: &str, port: u16, path: &str) -> Result<String> {
        // Validate protocol
        if !matches!(protocol, "http" | "https") {
            return Err(anyhow!("Invalid protocol: {}. Only http and https are allowed", protocol));
        }
        
        // Validate hostname
        Self::validate_hostname(hostname)?;
        
        // Validate port (u16 max is 65535, so only check for 0)
        if port == 0 {
            return Err(anyhow!("Invalid port: {}. Must be between 1 and 65535", port));
        }
        
        // Validate path
        let sanitized_path = Self::sanitize_url_path(path)?;
        
        // Construct and validate final URL
        let url_string = format!("{}://{}:{}{}", protocol, hostname, port, sanitized_path);
        let parsed_url = Url::parse(&url_string)
            .map_err(|e| anyhow!("Invalid URL construction: {}", e))?;
            
        // Additional security checks
        if let Some(host) = parsed_url.host_str() {
            if Self::is_private_ip(host) && protocol == "http" {
                warn!("HTTP connection to private IP detected: {}", host);
            }
        }
        
        Ok(url_string)
    }
    
    /// Sanitizes a filename to prevent path traversal attacks
    pub fn sanitize_filename(filename: &str) -> String {
        // Remove path separators and dangerous characters
        let mut sanitized = filename.to_string();
        for dangerous_char in ['/', '\\', '\0'] {
            sanitized = sanitized.replace(dangerous_char, "_");
        }
        sanitized = sanitized.replace("..", "_");

        let sanitized = sanitized
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(*c, '.' | '_' | '-'))
            .collect::<String>();

        // Ensure filename is not empty and has reasonable length
        if sanitized.is_empty() || sanitized.len() > 255 {
            format!("file_{}", uuid::Uuid::new_v4())
        } else {
            sanitized
        }
    }

    /// Validates hostname for security
    fn validate_hostname(hostname: &str) -> Result<()> {
        if hostname.is_empty() {
            return Err(anyhow!("Hostname cannot be empty"));
        }
        
        if hostname.len() > 253 {
            return Err(anyhow!("Hostname too long: {} chars", hostname.len()));
        }
        
        // Check for valid hostname characters
        let hostname_regex = Regex::new(r"^[a-zA-Z0-9.-]+$")
            .map_err(|e| anyhow!("Regex error: {}", e))?;
            
        if !hostname_regex.is_match(hostname) {
            return Err(anyhow!("Invalid hostname characters: {}", hostname));
        }
        
        // Check for suspicious patterns
        if hostname.contains("..") || hostname.starts_with('.') || hostname.ends_with('.') {
            return Err(anyhow!("Suspicious hostname pattern: {}", hostname));
        }
        
        Ok(())
    }
    
    /// Sanitizes URL path component
    fn sanitize_url_path(path: &str) -> Result<String> {
        if path.is_empty() {
            return Ok("/".to_string());
        }
        
        // Ensure path starts with /
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{}", path)
        };
        
        // Check for path traversal attempts
        if normalized_path.contains("..") || normalized_path.contains("//") {
            return Err(anyhow!("Path traversal attempt detected: {}", path));
        }
        
        // Validate path characters
        let path_regex = Regex::new(r"^[a-zA-Z0-9/_.-]+$")
            .map_err(|e| anyhow!("Regex error: {}", e))?;
            
        if !path_regex.is_match(&normalized_path) {
            return Err(anyhow!("Invalid path characters: {}", path));
        }
        
        Ok(normalized_path)
    }
    
    /// Checks for common injection patterns
    fn check_for_injection_patterns(input: &str) -> Result<()> {
        let dangerous_patterns = [
            // Command injection
            r";\s*(rm|sudo|curl|wget|nc|netcat)",
            r"\|\s*(rm|sudo|curl|wget|nc|netcat)",
            r"&&\s*(rm|sudo|curl|wget|nc|netcat)",
            r"\$\([^)]*\)",
            r"`[^`]*`",
            
            // SQL injection
            r"(?i)(union|select|insert|update|delete|drop|create|alter)\s+",
            r"(?i)(\-\-|\#|\/\*|\*\/)",
            r"(?i)(or|and)\s+\d+\s*=\s*\d+",
            
            // Script injection
            r"<script[^>]*>",
            r"javascript:",
            r"vbscript:",
            r"data:text/html",
            
            // Path traversal
            r"\.\./",
            r"\.\.\\",
        ];
        
        for pattern in &dangerous_patterns {
            let regex = Regex::new(pattern)
                .map_err(|e| anyhow!("Regex compilation error: {}", e))?;
                
            if regex.is_match(input) {
                return Err(anyhow!("Potentially dangerous pattern detected in input"));
            }
        }
        
        Ok(())
    }
    
    /// Validates file path for security
    pub fn validate_file_path(path: &str) -> Result<PathBuf> {
        if path.is_empty() {
            return Err(anyhow!("File path cannot be empty"));
        }

        let path_buf = PathBuf::from(path);

        // Check for path traversal
        if path.contains("..") {
            return Err(anyhow!("Path traversal attempt detected: {}", path));
        }

        // Check for absolute paths (may be dangerous)
        if path_buf.is_absolute() {
            warn!("Absolute path detected: {}", path);
        }

        // Validate file extension if present
        if let Some(extension) = path_buf.extension() {
            Self::validate_file_extension(extension.to_string_lossy().as_ref())?;
        }

        Ok(path_buf)
    }

    /// Comprehensive file upload validation
    pub async fn validate_file_upload(file_path: &std::path::Path) -> Result<()> {
        use tokio::fs;

        // Validate file path
        let path_str = file_path.to_string_lossy();
        Self::validate_file_path(&path_str)?;

        // Check if file exists
        if !file_path.exists() {
            return Err(anyhow!("File does not exist: {}", path_str));
        }

        // Check if it's actually a file (not a directory or symlink)
        let metadata = fs::metadata(file_path).await
            .map_err(|e| anyhow!("Cannot read file metadata: {}", e))?;

        if !metadata.is_file() {
            return Err(anyhow!("Path is not a regular file: {}", path_str));
        }

        // Check file size (100MB limit)
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow!(
                "File too large: {} bytes (max: {} bytes)",
                metadata.len(),
                MAX_FILE_SIZE
            ));
        }

        // Validate file extension
        if let Some(extension) = file_path.extension() {
            Self::validate_file_extension(extension.to_string_lossy().as_ref())?;
        } else {
            return Err(anyhow!("File has no extension: {}", path_str));
        }

        // Basic content validation for images
        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if matches!(ext_str.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp") {
                Self::validate_image_content(file_path).await?;
            }
        }

        Ok(())
    }

    /// Validates image file content for basic security
    async fn validate_image_content(file_path: &std::path::Path) -> Result<()> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(file_path).await
            .map_err(|e| anyhow!("Cannot open file for validation: {}", e))?;

        // Read first few bytes to check magic numbers
        let mut header = [0u8; 16];
        let bytes_read = file.read(&mut header).await
            .map_err(|e| anyhow!("Cannot read file header: {}", e))?;

        if bytes_read < 4 {
            return Err(anyhow!("File too small to be a valid image"));
        }

        // Check magic numbers for common image formats
        let is_valid_image = match &header[..4] {
            [0xFF, 0xD8, 0xFF, _] => true,  // JPEG
            [0x89, 0x50, 0x4E, 0x47] => true, // PNG
            [0x47, 0x49, 0x46, 0x38] => true, // GIF
            [0x42, 0x4D, _, _] => true,      // BMP
            [0x52, 0x49, 0x46, 0x46] => {   // WEBP (check for WEBP signature)
                bytes_read >= 12 && &header[8..12] == b"WEBP"
            }
            _ => false,
        };

        if !is_valid_image {
            return Err(anyhow!("File does not appear to be a valid image"));
        }

        Ok(())
    }

    /// Securely reads file content with size limits
    pub async fn read_file_securely(file_path: &std::path::Path) -> Result<Vec<u8>> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        // Validate file first
        Self::validate_file_upload(file_path).await?;

        let mut file = File::open(file_path).await
            .map_err(|e| anyhow!("Cannot open file: {}", e))?;

        let metadata = file.metadata().await
            .map_err(|e| anyhow!("Cannot read file metadata: {}", e))?;

        // Double-check size limit
        const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
        if metadata.len() > MAX_FILE_SIZE {
            return Err(anyhow!("File too large: {} bytes", metadata.len()));
        }

        let mut buffer = Vec::with_capacity(metadata.len() as usize);
        file.read_to_end(&mut buffer).await
            .map_err(|e| anyhow!("Cannot read file content: {}", e))?;

        debug!("Securely read file: {} bytes", buffer.len());
        Ok(buffer)
    }
    
    /// Validates file extension for security
    fn validate_file_extension(extension: &str) -> Result<()> {
        let allowed_extensions = [
            // Text files
            "txt", "md", "json", "yaml", "yml", "csv", "xml",
            // Images
            "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp",
            // Documents
            "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
            // Code files
            "rs", "py", "js", "ts", "html", "css", "sql",
        ];
        
        if !allowed_extensions.contains(&extension.to_lowercase().as_str()) {
            return Err(anyhow!("File extension not allowed: {}", extension));
        }
        
        Ok(())
    }
    
    /// Validates JSON payload structure
    pub fn validate_json_payload(payload: &serde_json::Value) -> Result<()> {
        // Check JSON depth to prevent stack overflow
        Self::check_json_depth(payload, 0, 10)?;
        
        // Check JSON size
        let serialized = serde_json::to_string(payload)
            .map_err(|e| anyhow!("JSON serialization error: {}", e))?;
            
        if serialized.len() > 10_000_000 { // 10MB limit
            return Err(anyhow!("JSON payload too large: {} bytes", serialized.len()));
        }
        
        Ok(())
    }
    
    /// Recursively checks JSON depth
    fn check_json_depth(value: &serde_json::Value, current_depth: usize, max_depth: usize) -> Result<()> {
        if current_depth > max_depth {
            return Err(anyhow!("JSON nesting too deep: {} levels", current_depth));
        }
        
        match value {
            serde_json::Value::Object(map) => {
                for (_, v) in map {
                    Self::check_json_depth(v, current_depth + 1, max_depth)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    Self::check_json_depth(v, current_depth + 1, max_depth)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// Checks if an IP address is private/local
    fn is_private_ip(host: &str) -> bool {
        if host == "localhost" || host == "127.0.0.1" || host == "::1" {
            return true;
        }
        
        // Check for private IP ranges
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            match ip {
                std::net::IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    // 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
                    octets[0] == 10 ||
                    (octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31) ||
                    (octets[0] == 192 && octets[1] == 168)
                }
                std::net::IpAddr::V6(_) => {
                    // For simplicity, consider all IPv6 as potentially private
                    true
                }
            }
        } else {
            false
        }
    }
    
    /// Sanitizes command input (for when command execution is enabled)
    pub fn sanitize_command_input(command: &str) -> Result<String> {
        // This should only be used if command execution is re-enabled with proper sandboxing
        if command.is_empty() {
            return Err(anyhow!("Command cannot be empty"));
        }
        
        if command.len() > 1000 {
            return Err(anyhow!("Command too long: {} chars", command.len()));
        }
        
        // Check for dangerous patterns
        Self::check_for_injection_patterns(command)?;
        
        // Additional command-specific checks
        let dangerous_commands = [
            "rm", "sudo", "su", "chmod", "chown", "mount", "umount",
            "dd", "mkfs", "fdisk", "kill", "killall", "reboot", "shutdown",
            "curl", "wget", "nc", "netcat", "ssh", "scp", "rsync",
        ];
        
        for dangerous in &dangerous_commands {
            if command.contains(dangerous) {
                return Err(anyhow!("Dangerous command detected: {}", dangerous));
            }
        }
        
        Ok(command.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_request_payload() {
        assert!(InputValidator::validate_request_payload("Hello world").is_ok());
        assert!(InputValidator::validate_request_payload("").is_err());
        assert!(InputValidator::validate_request_payload(&"x".repeat(2_000_000)).is_err());
    }
    
    #[test]
    fn test_validate_hostname() {
        assert!(InputValidator::validate_hostname("example.com").is_ok());
        assert!(InputValidator::validate_hostname("localhost").is_ok());
        assert!(InputValidator::validate_hostname("").is_err());
        assert!(InputValidator::validate_hostname("..example.com").is_err());
    }
    
    #[test]
    fn test_injection_detection() {
        assert!(InputValidator::check_for_injection_patterns("normal text").is_ok());
        assert!(InputValidator::check_for_injection_patterns("; rm -rf /").is_err());
        assert!(InputValidator::check_for_injection_patterns("$(malicious)").is_err());
        assert!(InputValidator::check_for_injection_patterns("SELECT * FROM users").is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(InputValidator::sanitize_filename("normal_file.txt"), "normal_file.txt");
        assert_eq!(InputValidator::sanitize_filename("file with spaces.txt"), "filewithspaces.txt");
        assert_eq!(InputValidator::sanitize_filename("../../../etc/passwd"), "______etc_passwd");
        assert_eq!(InputValidator::sanitize_filename("file<>:\"|?*.txt"), "file.txt");
        // Empty filename gets a UUID, so just check it's not empty
        assert!(!InputValidator::sanitize_filename("").is_empty());
    }

    #[test]
    fn test_validate_file_path() {
        assert!(InputValidator::validate_file_path("safe/file.txt").is_ok());
        assert!(InputValidator::validate_file_path("./local/file.txt").is_ok());
        assert!(InputValidator::validate_file_path("../../../etc/passwd").is_err());
        assert!(InputValidator::validate_file_path("").is_err());
    }

    #[test]
    fn test_sanitize_command_input() {
        assert!(InputValidator::sanitize_command_input("ls -la").is_ok());
        assert!(InputValidator::sanitize_command_input("").is_err());
        assert!(InputValidator::sanitize_command_input("rm -rf /").is_err());
        assert!(InputValidator::sanitize_command_input("sudo something").is_err());
    }
}
