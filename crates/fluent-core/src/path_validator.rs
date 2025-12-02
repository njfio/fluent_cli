use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

/// Centralized path validator for secure file operations
pub struct SecurePathValidator {
    allowed_roots: Vec<PathBuf>,
    allow_symlinks: bool,
    max_path_depth: usize,
}

impl SecurePathValidator {
    pub fn new(allowed_roots: Vec<String>) -> Self {
        Self {
            allowed_roots: allowed_roots.into_iter().map(PathBuf::from).collect(),
            allow_symlinks: false,
            max_path_depth: 20,
        }
    }

    pub fn with_symlinks(mut self, allow: bool) -> Self {
        self.allow_symlinks = allow;
        self
    }

    /// Validate a path and return the canonical version
    pub fn validate(&self, path: &str) -> Result<PathBuf> {
        // 1. Check for dangerous patterns
        if path.contains("..") {
            return Err(anyhow!("Path traversal detected: {}", path));
        }

        // 2. Canonicalize the path
        let canonical = self.canonicalize_path(path)?;

        // 3. Check path depth
        if canonical.components().count() > self.max_path_depth {
            return Err(anyhow!("Path exceeds maximum depth"));
        }

        // 4. Check against allowed roots
        self.check_allowed_roots(&canonical)?;

        // 5. Check symlinks if not allowed
        if !self.allow_symlinks {
            self.check_symlink(path, &canonical)?;
        }

        Ok(canonical)
    }

    fn canonicalize_path(&self, path: &str) -> Result<PathBuf> {
        let p = Path::new(path);
        if p.exists() {
            p.canonicalize().map_err(|e| anyhow!("Failed to canonicalize: {}", e))
        } else {
            // For non-existent files, canonicalize parent
            if let Some(parent) = p.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize()?;
                    if let Some(filename) = p.file_name() {
                        return Ok(canonical_parent.join(filename));
                    }
                }
            }
            // Fall back to current dir + path
            std::env::current_dir()?.join(p).canonicalize()
                .or_else(|_| Ok(std::env::current_dir()?.join(p)))
        }
    }

    fn check_allowed_roots(&self, canonical: &Path) -> Result<()> {
        if self.allowed_roots.is_empty() {
            return Ok(()); // No restrictions if empty
        }

        for root in &self.allowed_roots {
            let canonical_root = if root.exists() {
                root.canonicalize().unwrap_or_else(|_| root.clone())
            } else {
                root.clone()
            };

            if canonical.starts_with(&canonical_root) {
                return Ok(());
            }
        }

        Err(anyhow!("Path '{}' is not within allowed directories", canonical.display()))
    }

    fn check_symlink(&self, original: &str, _canonical: &Path) -> Result<()> {
        let original_path = Path::new(original);
        if original_path.exists() && original_path.is_symlink() {
            return Err(anyhow!("Symlinks are not allowed: {}", original));
        }
        Ok(())
    }
}

impl Default for SecurePathValidator {
    fn default() -> Self {
        Self::new(vec![
            ".".to_string(),
            "./src".to_string(),
            "./crates".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_traversal_blocked() {
        let validator = SecurePathValidator::default();
        assert!(validator.validate("../etc/passwd").is_err());
        assert!(validator.validate("foo/../../../etc/passwd").is_err());
    }

    #[test]
    fn test_valid_path() {
        let validator = SecurePathValidator::new(vec![".".to_string()]);
        // This should work for paths in current directory
        let result = validator.validate("./Cargo.toml");
        assert!(result.is_ok());
    }
}
