use async_trait::async_trait;
use emdash_core::{ApiError, StorageProvider};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Filesystem-backed storage scoped to a base directory.
/// The `STORAGE_PATH` environment variable sets the root (default: `./storage`).
pub struct LocalStorage {
    base: PathBuf,
}

impl LocalStorage {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self { base: base_path.as_ref().to_path_buf() }
    }

    /// Resolve and sanitise a logical path against the base directory.
    /// Rejects paths that try to escape with `..` components.
    fn resolve(&self, path: &str) -> Result<PathBuf, ApiError> {
        let rel = Path::new(path);
        // Reject absolute paths or path traversal attempts.
        if rel.is_absolute() || rel.components().any(|c| c == std::path::Component::ParentDir) {
            return Err(ApiError::BadRequest(format!(
                "invalid storage path: '{path}'"
            )));
        }
        Ok(self.base.join(rel))
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    async fn get_file(&self, path: &str) -> Result<Vec<u8>, ApiError> {
        let full = self.resolve(path)?;
        fs::read(&full).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ApiError::NotFound(path.into())
            } else {
                ApiError::Internal(e.to_string())
            }
        })
    }

    async fn put_file(&self, path: &str, data: &[u8]) -> Result<(), ApiError> {
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }
        fs::write(&full, data)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))
    }

    async fn delete_file(&self, path: &str) -> Result<(), ApiError> {
        let full = self.resolve(path)?;
        match fs::remove_file(&full).await {
            Ok(())                                                 => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e)                                                 => {
                Err(ApiError::Internal(e.to_string()))
            }
        }
    }

    async fn list_files(&self, prefix: &str) -> Result<Vec<String>, ApiError> {
        let dir = self.resolve(prefix)?;
        let mut names = Vec::new();
        let mut rd = match fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(names),
            Err(e) => return Err(ApiError::Internal(e.to_string())),
        };
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
        {
            if let Some(name) = entry.file_name().to_str() {
                names.push(format!("{prefix}/{name}"));
            }
        }
        Ok(names)
    }
}
