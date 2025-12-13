use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub enum ProjectIdSource {
    GitRemoteBranch,
    Path,
}

#[derive(Debug, Clone)]
pub struct ProjectIdentity {
    pub project_id: String,
    pub source: ProjectIdSource,
    pub repo_root: Option<PathBuf>,
    pub git_remote: Option<String>,
    pub git_branch: Option<String>,
}

/// Compute a stable project id for the current working directory.
///
/// Preference order:
/// 1) `sha256(git_remote + "#" + branch)` when available
/// 2) `sha256(absolute_repo_path)` fallback
///
/// This is intended for local-only scoping (never transmitted).
pub fn compute_project_identity() -> ProjectIdentity {
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    compute_project_identity_in(&base)
}

/// Compute a stable project id scoped to a specific directory.
pub fn compute_project_identity_in(dir: &Path) -> ProjectIdentity {
    let repo_root = git_stdout_in(dir, ["rev-parse", "--show-toplevel"]).map(PathBuf::from);

    let (git_remote, git_branch) = if repo_root.is_some() {
        let remote = git_stdout_in(dir, ["remote", "get-url", "origin"]).or_else(|| {
            git_stdout_in(dir, ["config", "--get", "remote.origin.url"]).filter(|s| !s.is_empty())
        });

        let mut branch = git_stdout_in(dir, ["rev-parse", "--abbrev-ref", "HEAD"]);
        if branch.as_deref() == Some("HEAD") {
            branch = git_stdout_in(dir, ["rev-parse", "HEAD"]);
        }

        (remote, branch)
    } else {
        (None, None)
    };

    let (seed, source) =
        if let (Some(remote), Some(branch)) = (git_remote.clone(), git_branch.clone()) {
            (
                format!("{}#{}", remote, branch),
                ProjectIdSource::GitRemoteBranch,
            )
        } else {
            let abs = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
            (abs.to_string_lossy().to_string(), ProjectIdSource::Path)
        };

    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = hasher.finalize();

    ProjectIdentity {
        project_id: hex::encode(digest),
        source,
        repo_root,
        git_remote,
        git_branch,
    }
}

fn git_stdout_in<I, S>(dir: &Path, args: I) -> Option<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_non_empty_id() {
        let id = compute_project_identity();
        assert!(!id.project_id.is_empty());
    }

    #[test]
    fn prefers_git_remote_and_branch_when_available() {
        let tmp = tempfile::tempdir().expect("tempdir");

        // Initialize a tiny git repo
        assert!(Command::new("git")
            .current_dir(tmp.path())
            .args(["init", "-q"])
            .status()
            .expect("git init")
            .success());

        // Create an initial commit
        std::fs::write(tmp.path().join("README.txt"), "hello").expect("write");
        assert!(Command::new("git")
            .current_dir(tmp.path())
            .args(["add", "."])
            .status()
            .expect("git add")
            .success());
        assert!(Command::new("git")
            .current_dir(tmp.path())
            .args([
                "-c",
                "user.email=test@example.com",
                "-c",
                "user.name=test",
                "commit",
                "-m",
                "init",
                "-q",
            ])
            .status()
            .expect("git commit")
            .success());

        // Set a remote and ensure we get GitRemoteBranch source
        assert!(Command::new("git")
            .current_dir(tmp.path())
            .args(["remote", "add", "origin", "https://example.com/repo.git"])
            .status()
            .expect("git remote add")
            .success());

        let ident = compute_project_identity_in(tmp.path());
        assert!(matches!(ident.source, ProjectIdSource::GitRemoteBranch));
        assert!(ident.git_remote.is_some());
        assert!(ident.git_branch.is_some());
    }
}
