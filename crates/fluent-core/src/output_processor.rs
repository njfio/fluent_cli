use anyhow::{anyhow, Context, Result};
use log::{debug, info};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::env;
use std::path::Path;
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

pub struct OutputProcessor;

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

        let client = Client::new();
        let response = client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file: HTTP status {}",
                response.status()
            ));
        }

        let file_name = if let Some(name) = suggested_name {
            name
        } else {
            Url::parse(url)?
                .path_segments()
                .and_then(|segments| segments.last())
                .map(|s| s.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string())
        };

        let file_path = directory.join(&file_name);
        let content = response.bytes().await?;
        fs::write(&file_path, &content).await?;

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
        let code_block_regex = Regex::new(r"```(?:\w+)?\n([\s\S]*?)\n```").unwrap();
        code_block_regex
            .captures_iter(content)
            .filter_map(|cap| cap.get(1))
            .map(|m| m.as_str().trim().to_string())
            .collect()
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
        // Use a platform-agnostic way to get the temp directory
        let temp_dir = env::temp_dir();
        let file_name = format!(
            "script_{}.{}",
            Uuid::new_v4(),
            if cfg!(windows) { "bat" } else { "sh" }
        );
        let temp_file = temp_dir.join(file_name);

        // Write the script to the temporary file
        fs::write(&temp_file, script)
            .await
            .context("Failed to write script to temporary file")?;

        // Set executable permissions on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&temp_file).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&temp_file, perms)
                .await
                .context("Failed to set file permissions")?;
        }

        // Execute the script
        let result = if cfg!(windows) {
            Command::new("cmd").arg("/C").arg(&temp_file).output().await
        } else {
            Command::new("sh").arg(&temp_file).output().await
        }
        .context("Failed to execute script")?;

        // Remove the temporary file
        fs::remove_file(&temp_file)
            .await
            .context("Failed to remove temporary file")?;

        // Collect and format the output
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        Ok(format!("Script Output:\n{}\nErrors:\n{}\n", stdout, stderr))
    }

    async fn execute_commands(commands: &str) -> Result<String> {
        let mut output = String::new();
        for command in commands.lines() {
            let trimmed = command.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            output.push_str(&format!("Executing: {}\n", trimmed));

            let result = Command::new("sh").arg("-c").arg(trimmed).output().await?;

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
            self.skin.paragraph.set_fg(Color::Rgb {
                r: theme.settings.foreground.unwrap().r,
                g: theme.settings.foreground.unwrap().g,
                b: theme.settings.foreground.unwrap().b,
            });
            Ok(())
        } else {
            Err(anyhow::anyhow!("Theme not found"))
        }
    }
}

pub fn format_markdown(content: &str) -> Result<String> {
    debug!("format_markdown");
    let formatter = MarkdownFormatter::new();
    formatter.format(content)
}
