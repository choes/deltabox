use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use walkdir::WalkDir;

use crate::storage::StorageBackend;

#[derive(Debug, Clone)]
pub struct LocalStorage {
    backend_id: String,
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(backend_id: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            backend_id: backend_id.into(),
            root: root.into(),
        }
    }

    pub fn object_key_for_hash(hash: &str) -> String {
        let prefix = hash.get(0..2).unwrap_or("00");
        let rest = hash.get(2..).unwrap_or(hash);
        format!("{prefix}/{rest}")
    }

    pub fn list_object_keys(&self) -> Result<Vec<String>> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut keys = Vec::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                let relative = entry.path().strip_prefix(&self.root)?;
                keys.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
        Ok(keys)
    }

    pub fn delete_object_key(&self, object_key: &str) -> Result<()> {
        let path = self.root.join(object_key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn path_for_chunk_id(&self, chunk_id: &str) -> Result<PathBuf> {
        let hash = chunk_id
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow!("unsupported chunk id: {chunk_id}"))?;
        Ok(self.root.join(Self::object_key_for_hash(hash)))
    }
}

impl StorageBackend for LocalStorage {
    fn backend_id(&self) -> &str {
        &self.backend_id
    }

    fn put_chunk(&self, chunk_id: &str, data: &[u8]) -> Result<()> {
        let path = self.path_for_chunk_id(chunk_id)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            fs::write(&path, data)
                .with_context(|| format!("failed to write chunk: {}", path.display()))?;
        }
        Ok(())
    }

    fn get_chunk(&self, chunk_id: &str) -> Result<Vec<u8>> {
        let path = self.path_for_chunk_id(chunk_id)?;
        fs::read(&path).with_context(|| format!("failed to read chunk: {}", path.display()))
    }

    fn has_chunk(&self, chunk_id: &str) -> Result<bool> {
        Ok(self.path_for_chunk_id(chunk_id)?.exists())
    }

    fn delete_chunk(&self, chunk_id: &str) -> Result<()> {
        let path = self.path_for_chunk_id(chunk_id)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}
