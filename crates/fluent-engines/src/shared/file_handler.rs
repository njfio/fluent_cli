use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as Base64Engine;
use fluent_core::input_validator::InputValidator;
use reqwest::multipart::{Form, Part};
use std::path::Path;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

/// Shared file handling utilities for engines
pub struct FileHandler;

impl FileHandler {
    /// Read and encode file as base64
    pub async fn encode_file_base64(file_path: &Path) -> Result<String> {
        // Security validation
        InputValidator::validate_file_upload(file_path).await?;

        // Use secure file reading
        let buffer = InputValidator::read_file_securely(file_path).await?;
        let base64_content = STANDARD.encode(&buffer);
        Ok(base64_content)
    }

    /// Create multipart form with file upload
    pub async fn create_file_form(
        file_path: &Path,
        purpose: &str,
        additional_fields: Option<&[(&str, &str)]>,
    ) -> Result<Form> {
        // Security validation
        InputValidator::validate_file_upload(file_path).await?;

        let file_name = file_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("File name is not valid UTF-8"))?;

        // Sanitize filename
        let sanitized_filename = InputValidator::sanitize_filename(file_name);

        let file = File::open(file_path).await.context("Failed to open file")?;
        let stream = FramedRead::new(file, BytesCodec::new());
        let file_part =
            Part::stream(reqwest::Body::wrap_stream(stream)).file_name(sanitized_filename);

        let mut form = Form::new()
            .part("file", file_part)
            .text("purpose", purpose.to_string());

        // Add additional fields if provided
        if let Some(fields) = additional_fields {
            for (key, value) in fields {
                form = form.text(key.to_string(), value.to_string());
            }
        }

        Ok(form)
    }

    /// Get file extension
    pub fn get_file_extension(file_path: &Path) -> Option<String> {
        file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }

    /// Determine MIME type from file extension
    pub fn get_mime_type(file_path: &Path) -> String {
        match Self::get_file_extension(file_path).as_deref() {
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("webp") => "image/webp".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("txt") => "text/plain".to_string(),
            Some("json") => "application/json".to_string(),
            Some("xml") => "application/xml".to_string(),
            Some("csv") => "text/csv".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    /// Check if file is an image
    pub fn is_image_file(file_path: &Path) -> bool {
        matches!(
            Self::get_file_extension(file_path).as_deref(),
            Some("jpg") | Some("jpeg") | Some("png") | Some("gif") | Some("webp") | Some("bmp")
        )
    }

    /// Check if file is a document
    pub fn is_document_file(file_path: &Path) -> bool {
        matches!(
            Self::get_file_extension(file_path).as_deref(),
            Some("pdf") | Some("doc") | Some("docx") | Some("txt") | Some("rtf")
        )
    }

    /// Get image format for base64 data URL
    pub fn get_image_format(file_path: &Path) -> String {
        match Self::get_file_extension(file_path).as_deref() {
            Some("jpg") | Some("jpeg") => "jpeg".to_string(),
            Some("png") => "png".to_string(),
            Some("gif") => "gif".to_string(),
            Some("webp") => "webp".to_string(),
            _ => "png".to_string(), // Default fallback
        }
    }

    /// Create data URL from file
    pub async fn create_data_url(file_path: &Path) -> Result<String> {
        let base64_content = Self::encode_file_base64(file_path).await?;
        let mime_type = Self::get_mime_type(file_path);
        Ok(format!("data:{};base64,{}", mime_type, base64_content))
    }

    /// Validate file size
    pub async fn validate_file_size(file_path: &Path, max_size_mb: u64) -> Result<()> {
        let metadata = tokio::fs::metadata(file_path)
            .await
            .context("Failed to read file metadata")?;

        let file_size_mb = metadata.len() / (1024 * 1024);

        if file_size_mb > max_size_mb {
            return Err(anyhow::anyhow!(
                "File size ({} MB) exceeds maximum allowed size ({} MB)",
                file_size_mb,
                max_size_mb
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_file_extension() {
        let path = PathBuf::from("test.jpg");
        assert_eq!(
            FileHandler::get_file_extension(&path),
            Some("jpg".to_string())
        );

        let path = PathBuf::from("test.PNG");
        assert_eq!(
            FileHandler::get_file_extension(&path),
            Some("png".to_string())
        );

        let path = PathBuf::from("test");
        assert_eq!(FileHandler::get_file_extension(&path), None);
    }

    #[test]
    fn test_get_mime_type() {
        let path = PathBuf::from("test.jpg");
        assert_eq!(FileHandler::get_mime_type(&path), "image/jpeg");

        let path = PathBuf::from("test.png");
        assert_eq!(FileHandler::get_mime_type(&path), "image/png");

        let path = PathBuf::from("test.pdf");
        assert_eq!(FileHandler::get_mime_type(&path), "application/pdf");

        let path = PathBuf::from("test.unknown");
        assert_eq!(
            FileHandler::get_mime_type(&path),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_is_image_file() {
        assert!(FileHandler::is_image_file(&PathBuf::from("test.jpg")));
        assert!(FileHandler::is_image_file(&PathBuf::from("test.png")));
        assert!(!FileHandler::is_image_file(&PathBuf::from("test.pdf")));
        assert!(!FileHandler::is_image_file(&PathBuf::from("test.txt")));
    }

    #[test]
    fn test_is_document_file() {
        assert!(FileHandler::is_document_file(&PathBuf::from("test.pdf")));
        assert!(FileHandler::is_document_file(&PathBuf::from("test.txt")));
        assert!(!FileHandler::is_document_file(&PathBuf::from("test.jpg")));
        assert!(!FileHandler::is_document_file(&PathBuf::from("test.png")));
    }

    #[test]
    fn test_get_image_format() {
        assert_eq!(
            FileHandler::get_image_format(&PathBuf::from("test.jpg")),
            "jpeg"
        );
        assert_eq!(
            FileHandler::get_image_format(&PathBuf::from("test.png")),
            "png"
        );
        assert_eq!(
            FileHandler::get_image_format(&PathBuf::from("test.unknown")),
            "png"
        );
    }
}
