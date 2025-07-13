use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};
use termimad::crossterm::style::Color;
use termimad::{MadSkin, StyledChar};
use tokio::fs;
use tokio::process::Command;
use url::Url;
use uuid::Uuid;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct OutputProcessor;

impl OutputProcessor {
    /// Validates that a URL is safe for downloading
    fn validate_download_url(url: &str) -> Result<()> {
        let parsed_url = Url::parse(url)?;

        // Only allow HTTPS for security
        if parsed_url.scheme() != "https" {
            return Err(anyhow!("Only HTTPS URLs are allowed for downloads"));
        }

        // Block localhost and private IP ranges
        if let Some(host) = parsed_url.host_str() {
            if host == "localhost"
                || host == "127.0.0.1"
                || host.starts_with("192.168.")
                || host.starts_with("10.")
                || host.starts_with("172.")
            {
                return Err(anyhow!(
                    "Downloads from private/local addresses are not allowed"
                ));
            }
        }

        Ok(())
    }

    /// Sanitizes a filename to prevent path traversal attacks
    fn sanitize_filename(filename: &str) -> String {
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
            format!("file_{}", Uuid::new_v4())
        } else {
            sanitized
        }
    }

    /// Creates a secure temporary file with restricted permissions
    async fn create_secure_temp_file(content: &[u8], extension: &str) -> Result<PathBuf> {
        let temp_dir = env::temp_dir();
        let file_name = format!(
            "fluent_{}_{}.{}",
            std::process::id(),
            Uuid::new_v4(),
            Self::sanitize_filename(extension)
        );
        let temp_file = temp_dir.join(file_name);

        // Create file with secure permissions (owner read/write only)
        fs::write(&temp_file, content).await?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&temp_file).await?.permissions();
            perms.set_mode(0o600); // Owner read/write only
            fs::set_permissions(&temp_file, perms).await?;
        }

        Ok(temp_file)
    }
}

impl OutputProcessor {
    pub async fn download_media_files(content: &str, directory: &Path) -> Result<()> {
        debug!("Starting media file download process");

        // Try to parse the content as JSON
        if let Ok(json_content) = serde_json::from_str::<Value>(content) {
            debug!("Content parsed as JSON, attempting to download from JSON structure");
            Self::download_from_json(&json_content, directory).await?;
        } else {
            debug!("Content is not valid JSON, proceeding with regex-based URL extraction");
            // Corrected regex for URL matching, including query parameters
            let url_regex = Regex::new(
                r#"(https?://[^\s"']+\.(?:jpg|jpeg|png|gif|bmp|svg|mp4|webm|ogg)(?:\?[^\s"']+)?)"#,
            )?;

            for cap in url_regex.captures_iter(content) {
                let url = &cap[1]; // This includes the full URL with query parameters
                debug!("Found URL in content: {}", url);
                Self::download_file(url, directory, None).await?;
            }
        }

        Ok(())
    }

    async fn download_file(
        url: &str,
        directory: &Path,
        suggested_name: Option<String>,
    ) -> Result<()> {
        debug!("Attempting to download file from URL: {}", url);

        // Security validation
        Self::validate_download_url(url)?;

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file: HTTP status {}",
                response.status()
            ));
        }

        // Check content length to prevent excessive downloads
        if let Some(content_length) = response.content_length() {
            if content_length > 100 * 1024 * 1024 {
                // 100MB limit
                return Err(anyhow!("File too large: {} bytes", content_length));
            }
        }

        let file_name = if let Some(name) = suggested_name {
            Self::sanitize_filename(&name)
        } else {
            let extracted_name = Url::parse(url)?
                .path_segments()
                .and_then(|segments| segments.last())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("download_{}", Uuid::new_v4()));
            Self::sanitize_filename(&extracted_name)
        };

        let file_path = directory.join(&file_name);
        let content = response.bytes().await?;

        // Create file with secure permissions
        fs::write(&file_path, &content).await?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&file_path).await?.permissions();
            perms.set_mode(0o644); // Owner read/write, group/others read only
            fs::set_permissions(&file_path, perms).await?;
        }

        info!(
            "Downloaded: {} ({} bytes)",
            file_path.display(),
            content.len()
        );

        Ok(())
    }

    async fn download_from_json(json_content: &Value, directory: &Path) -> Result<()> {
        if let Some(data) = json_content.get("data") {
            if let Some(data_array) = data.as_array() {
                for item in data_array {
                    if let Some(url) = item.get("url").and_then(|u| u.as_str()) {
                        let file_name = Self::extract_file_name_from_url(url);
                        Self::download_file(url, directory, file_name).await?;
                    }
                }
            }
        }
        Ok(())
    }

    fn extract_file_name_from_url(url: &str) -> Option<String> {
        Url::parse(url)
            .ok()?
            .path_segments()?
            .last()?
            .split('?')
            .next()
            .map(|s| s.to_string())
    }

    pub fn parse_code(content: &str) -> Vec<String> {
        match Regex::new(r"```(?:\w+)?\n([\s\S]*?)\n```") {
            Ok(code_block_regex) => code_block_regex
                .captures_iter(content)
                .filter_map(|cap| cap.get(1))
                .map(|m| m.as_str().trim().to_string())
                .collect(),
            Err(e) => {
                debug!("Failed to create regex for code block parsing: {}", e);
                Vec::new()
            }
        }
    }

    pub async fn execute_code(content: &str) -> Result<String> {
        let code_blocks = Self::parse_code(content);
        let mut output = String::new();
        for block in code_blocks {
            output.push_str(&format!("Executing code block:\n```\n{}\n```\n", block));

            if Self::is_script(&block) {
                output.push_str(&Self::execute_script(&block).await?);
            } else {
                output.push_str(&Self::execute_commands(&block).await?);
            }

            output.push('\n');
        }
        Ok(output)
    }

    fn is_script(block: &str) -> bool {
        block.starts_with("#!/bin/") || block.starts_with("#!/usr/bin/env")
    }

    async fn execute_script(script: &str) -> Result<String> {
        // Enhanced security implementation for script execution

        // Input validation
        if script.len() > 50_000 {
            return Err(anyhow!("Script too large (max 50KB)"));
        }

        // Check for dangerous patterns
        let dangerous_patterns = [
            "rm -rf", "sudo", "chmod", "chown", "passwd", "su ",
            "eval", "exec", "system", "shell_exec", "passthru",
            "curl", "wget", "nc ", "netcat", "telnet", "ssh",
            "/etc/", "/proc/", "/sys/", "/dev/", "/root/",
            "import os", "import subprocess", "import sys",
        ];

        for pattern in &dangerous_patterns {
            if script.to_lowercase().contains(pattern) {
                return Err(anyhow!("Script contains potentially dangerous pattern: {}", pattern));
            }
        }

        // Check if script execution is explicitly enabled via environment variable
        if std::env::var("FLUENT_ENABLE_SCRIPT_EXECUTION").unwrap_or_default() != "true" {
            return Err(anyhow!(
                "Script execution is disabled for security. Set FLUENT_ENABLE_SCRIPT_EXECUTION=true to enable."
            ));
        }

        // Create secure temporary file for script execution
        let temp_file = Self::create_secure_temp_file(
            script.as_bytes(),
            if cfg!(windows) { "bat" } else { "sh" },
        )
        .await?;

        // Set executable permissions on Unix-like systems
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&temp_file).await?.permissions();
            perms.set_mode(0o700); // Owner execute only
            fs::set_permissions(&temp_file, perms).await?;
        }

        // Execute with timeout and resource limits
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            if cfg!(windows) {
                Command::new("cmd").arg("/C").arg(&temp_file).output()
            } else {
                Command::new("sh").arg(&temp_file).output()
            },
        )
        .await
        .context("Script execution timed out")?
        .context("Failed to execute script")?;

        // Clean up temporary file
        let _ = fs::remove_file(&temp_file).await;

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        Ok(format!("Script Output:\n{}\nErrors:\n{}\n", stdout, stderr))
    }

    async fn execute_commands(commands: &str) -> Result<String> {
        // Enhanced security implementation for command execution

        // Input validation
        if commands.len() > 10_000 {
            return Err(anyhow!("Command input too large (max 10KB)"));
        }

        // Get configurable command whitelist from environment or use defaults
        let safe_commands = Self::get_command_whitelist();

        // Validate each command against security policies
        for command in commands.lines() {
            let trimmed = command.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // First validate against security patterns
            Self::validate_command_security(trimmed)?;

            // Then check against whitelist
            let cmd_parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(cmd) = cmd_parts.first() {
                if !safe_commands.contains(cmd) {
                    return Err(anyhow!("Command '{}' not in whitelist", cmd));
                }
            }
        }

        // Check if command execution is explicitly enabled via environment variable
        if std::env::var("FLUENT_ENABLE_COMMAND_EXECUTION").unwrap_or_default() != "true" {
            return Err(anyhow!(
                "Command execution is disabled for security. Set FLUENT_ENABLE_COMMAND_EXECUTION=true to enable."
            ));
        }

        let mut output = String::new();
        for command in commands.lines() {
            let trimmed = command.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Enhanced command validation
            let cmd_parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let Some(cmd) = cmd_parts.first() {
                if !safe_commands.contains(cmd) {
                    output.push_str(&format!("Blocked non-whitelisted command: {}\n", trimmed));
                    continue;
                }
            }

            output.push_str(&format!("Executing: {}\n", trimmed));

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                Command::new("sh")
                    .arg("-c")
                    .arg(trimmed)
                    .env_clear() // Clear environment variables
                    .env("PATH", "/usr/bin:/bin") // Minimal PATH
                    .output(),
            )
            .await
            .context("Command execution timed out")?
            .context("Failed to execute command")?;

            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);

            output.push_str(&format!("Output:\n{}\n", stdout));
            if !stderr.is_empty() {
                output.push_str(&format!("Errors:\n{}\n", stderr));
            }
            output.push('\n');
        }
        Ok(output)
    }

    /// Get configurable command whitelist from environment or defaults
    fn get_command_whitelist() -> Vec<&'static str> {
        // Check if custom whitelist is provided via environment variable
        if let Ok(custom_commands) = std::env::var("FLUENT_ALLOWED_COMMANDS") {
            // Parse comma-separated list and return as static references
            // For now, return default list and log the custom commands
            log::info!("Custom command whitelist provided: {}", custom_commands);
            // TODO: Implement dynamic whitelist parsing with proper lifetime management
        }

        // Default safe command whitelist
        vec![
            "echo", "cat", "ls", "pwd", "date", "whoami", "id",
            "head", "tail", "wc", "grep", "sort", "uniq", "find",
            "which", "type", "file", "stat", "du", "df"
        ]
    }

    /// Validate command against security policies
    fn validate_command_security(command: &str) -> Result<()> {
        // Check command length
        if command.len() > 1000 {
            return Err(anyhow!("Command too long (max 1000 characters)"));
        }

        // Check for dangerous patterns
        let dangerous_patterns = [
            "rm ", "rmdir", "del ", "format", "mkfs",
            "dd ", "fdisk", "parted", "mount", "umount",
            "sudo", "su ", "chmod +x", "chown", "chgrp",
            "curl", "wget", "nc ", "netcat", "telnet",
            "ssh", "scp", "rsync", "ftp", "sftp",
            "python", "perl", "ruby", "node", "php",
            "bash", "sh ", "zsh", "fish", "csh",
            "eval", "exec", "source", ".", "$(", "`",
            "&&", "||", ";", "|", ">", ">>", "<",
            "kill", "killall", "pkill", "nohup", "&"
        ];

        for pattern in &dangerous_patterns {
            if command.to_lowercase().contains(pattern) {
                return Err(anyhow!("Command contains dangerous pattern: {}", pattern));
            }
        }

        Ok(())
    }
}

pub struct MarkdownFormatter {
    skin: MadSkin,
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        let mut skin = MadSkin::default();
        skin.set_bg(Color::Rgb {
            r: 10,
            g: 10,
            b: 10,
        });
        skin.set_headers_fg(Color::Rgb {
            r: 255,
            g: 187,
            b: 0,
        });
        skin.bold.set_fg(Color::Rgb {
            r: 255,
            g: 215,
            b: 0,
        });
        skin.italic.set_fg(Color::Rgb {
            r: 173,
            g: 216,
            b: 230,
        });
        skin.paragraph.set_fg(Color::Rgb {
            r: 220,
            g: 220,
            b: 220,
        }); // Light grey for normal text
        skin.bullet = StyledChar::from_fg_char(Color::Rgb { r: 0, g: 255, b: 0 }, '•');

        // Set code block colors
        skin.code_block.set_bg(Color::Rgb {
            r: 30,
            g: 30,
            b: 30,
        }); // Slightly lighter than main background
        skin.code_block.set_fg(Color::Rgb {
            r: 255,
            g: 255,
            b: 255,
        }); // White text for code

        MarkdownFormatter {
            skin,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }
}
impl MarkdownFormatter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn format(&self, content: &str) -> Result<String> {
        debug!("Formatting markdown");
        let mut output = String::new();
        let lines: Vec<&str> = content.lines().collect();

        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();

        for line in lines {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    output.push_str(&self.highlight_code(&code_block_lang, &code_block_content)?);
                    output.push_str("\n\n");
                    in_code_block = false;
                    code_block_content.clear();
                } else {
                    // Start of code block
                    in_code_block = true;
                    code_block_lang = line.trim_start_matches('`').to_string();
                }
            } else if in_code_block {
                code_block_content.push_str(line);
                code_block_content.push('\n');
            } else {
                // Use push_str to append the formatted line to our String
                output.push_str(&self.skin.inline(line).to_string());
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn highlight_code(&self, lang: &str, code: &str) -> Result<String> {
        debug!("highlight_code: {}", lang);
        let syntax = self
            .syntax_set
            .find_syntax_by_extension(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut highlighter =
            HighlightLines::new(syntax, &self.theme_set.themes["base16-ocean.dark"]);

        let mut output = String::new();
        for line in LinesWithEndings::from(code) {
            let ranges: Vec<(Style, &str)> = highlighter.highlight_line(line, &self.syntax_set)?;
            let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
            output.push_str(&escaped);
        }

        Ok(output)
    }

    pub fn set_theme(&mut self, theme_name: &str) -> Result<()> {
        debug!("set_theme: {}", theme_name);
        if let Some(theme) = self.theme_set.themes.get(theme_name) {
            if let Some(foreground) = theme.settings.foreground {
                self.skin.paragraph.set_fg(Color::Rgb {
                    r: foreground.r,
                    g: foreground.g,
                    b: foreground.b,
                });
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Theme '{}' has no foreground color defined",
                    theme_name
                ))
            }
        } else {
            Err(anyhow::anyhow!("Theme '{}' not found", theme_name))
        }
    }
}

pub fn format_markdown(content: &str) -> Result<String> {
    debug!("format_markdown");
    let formatter = MarkdownFormatter::new();
    formatter.format(content)
}
