use std::path::PathBuf;

use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};

use crate::storage::local::LocalStorage;
use crate::storage::s3::{S3Storage, S3StorageConfig};
use crate::storage::StorageBackend;
use crate::util::now_rfc3339;
use crate::{StorageBackendRecord, Vault};

impl Vault {
    pub fn add_local_backend(
        &self,
        backend_id: &str,
        path: PathBuf,
    ) -> Result<StorageBackendRecord> {
        validate_backend_id(backend_id)?;
        let config_json = serde_json::json!({
            "path": path.to_string_lossy()
        })
        .to_string();
        let now = now_rfc3339();
        let record = StorageBackendRecord {
            backend_id: backend_id.to_owned(),
            backend_type: "local".to_owned(),
            config_json,
            status: "available".to_owned(),
            created_at: now.clone(),
            updated_at: now,
        };
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO storage_backends (backend_id, backend_type, config_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(backend_id) DO UPDATE SET
                backend_type = excluded.backend_type,
                config_json = excluded.config_json,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                record.backend_id,
                record.backend_type,
                record.config_json,
                record.status,
                record.created_at,
                record.updated_at,
            ],
        )?;
        self.record_event(
            "backend.upserted",
            Some(&record.backend_id),
            Some(&record.backend_type),
        )?;
        Ok(record)
    }

    pub fn list_backends(&self) -> Result<Vec<StorageBackendRecord>> {
        self.ensure_default_local_backend()?;
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT backend_id, backend_type, config_json, status, created_at, updated_at
             FROM storage_backends
             ORDER BY backend_id",
        )?;
        let rows = stmt.query_map([], row_to_backend_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn add_s3_backend(
        &self,
        backend_id: &str,
        endpoint: String,
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        prefix: Option<String>,
        allow_http: bool,
        path_style: bool,
    ) -> Result<StorageBackendRecord> {
        validate_backend_id(backend_id)?;
        let config_json = serde_json::json!({
            "endpoint": endpoint,
            "bucket": bucket,
            "region": region,
            "prefix": prefix,
            "allow_http": allow_http,
            "path_style": path_style
        })
        .to_string();
        let now = now_rfc3339();
        let record = StorageBackendRecord {
            backend_id: backend_id.to_owned(),
            backend_type: "s3".to_owned(),
            config_json,
            status: "available".to_owned(),
            created_at: now.clone(),
            updated_at: now,
        };
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO storage_backends (backend_id, backend_type, config_json, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(backend_id) DO UPDATE SET
                backend_type = excluded.backend_type,
                config_json = excluded.config_json,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                record.backend_id,
                record.backend_type,
                record.config_json,
                record.status,
                record.created_at,
                record.updated_at,
            ],
        )?;
        self.record_event(
            "backend.upserted",
            Some(&record.backend_id),
            Some(&record.backend_type),
        )?;
        self.set_backend_secret(&record.backend_id, "access_key", &access_key)?;
        self.set_backend_secret(&record.backend_id, "secret_key", &secret_key)?;
        Ok(record)
    }

    pub fn local_backend_by_id(&self, backend_id: &str) -> Result<LocalStorage> {
        if backend_id == "local" {
            self.ensure_default_local_backend()?;
        }
        let record = self
            .find_backend(backend_id)?
            .ok_or_else(|| anyhow!("backend not found: {backend_id}"))?;
        if record.backend_type != "local" {
            return Err(anyhow!("backend is not local: {backend_id}"));
        }
        let value: serde_json::Value = serde_json::from_str(&record.config_json)?;
        let path = value
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("local backend missing path: {backend_id}"))?;
        Ok(LocalStorage::new(record.backend_id, PathBuf::from(path)))
    }

    pub fn storage_backend_by_id(&self, backend_id: &str) -> Result<Box<dyn StorageBackend>> {
        if backend_id == "local" {
            self.ensure_default_local_backend()?;
        }
        let record = self
            .find_backend(backend_id)?
            .ok_or_else(|| anyhow!("backend not found: {backend_id}"))?;
        match record.backend_type.as_str() {
            "local" => {
                let value: serde_json::Value = serde_json::from_str(&record.config_json)?;
                let path = value
                    .get("path")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| anyhow!("local backend missing path: {backend_id}"))?;
                Ok(Box::new(LocalStorage::new(
                    record.backend_id,
                    PathBuf::from(path),
                )))
            }
            "s3" => {
                let value: serde_json::Value = serde_json::from_str(&record.config_json)?;
                let config = S3StorageConfig {
                    endpoint: json_string(&value, "endpoint", backend_id)?,
                    bucket: json_string(&value, "bucket", backend_id)?,
                    region: json_string(&value, "region", backend_id)?,
                    access_key: self.get_backend_secret(backend_id, "access_key")?,
                    secret_key: self.get_backend_secret(backend_id, "secret_key")?,
                    prefix: value
                        .get("prefix")
                        .and_then(|value| value.as_str())
                        .map(ToOwned::to_owned),
                    allow_http: value
                        .get("allow_http")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                    path_style: value
                        .get("path_style")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(true),
                };
                Ok(Box::new(S3Storage::new(record.backend_id, config)?))
            }
            other => Err(anyhow!("unsupported backend type {other}: {backend_id}")),
        }
    }

    pub(crate) fn ensure_default_local_backend(&self) -> Result<()> {
        if self.find_backend("local")?.is_some() {
            return Ok(());
        }
        self.add_local_backend("local", self.chunk_dir.clone())?;
        Ok(())
    }

    pub(crate) fn object_key_for_backend_chunk(
        &self,
        backend_id: &str,
        chunk_id: &str,
    ) -> Result<String> {
        let record = self
            .find_backend(backend_id)?
            .ok_or_else(|| anyhow!("backend not found: {backend_id}"))?;
        let hash = chunk_id
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow!("unsupported chunk id: {chunk_id}"))?;
        let key = match record.backend_type.as_str() {
            "local" => LocalStorage::object_key_for_hash(hash),
            "s3" => {
                let value: serde_json::Value = serde_json::from_str(&record.config_json)?;
                let object_key = S3Storage::object_key_for_hash(hash);
                match value.get("prefix").and_then(|value| value.as_str()) {
                    Some(prefix) if !prefix.is_empty() => {
                        format!("{}/{}", prefix.trim_matches('/'), object_key)
                    }
                    _ => object_key,
                }
            }
            other => return Err(anyhow!("unsupported backend type {other}: {backend_id}")),
        };
        Ok(key)
    }

    fn find_backend(&self, backend_id: &str) -> Result<Option<StorageBackendRecord>> {
        let conn = self.open_db()?;
        conn.query_row(
            "SELECT backend_id, backend_type, config_json, status, created_at, updated_at
             FROM storage_backends WHERE backend_id = ?1",
            params![backend_id],
            row_to_backend_record,
        )
        .optional()
        .map_err(Into::into)
    }
}

fn json_string(value: &serde_json::Value, key: &str, backend_id: &str) -> Result<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("backend {backend_id} missing {key}"))
}

fn validate_backend_id(backend_id: &str) -> Result<()> {
    if backend_id.trim().is_empty() {
        return Err(anyhow!("backend id cannot be empty"));
    }
    if backend_id.contains(char::is_whitespace) {
        return Err(anyhow!("backend id cannot contain whitespace"));
    }
    Ok(())
}

fn row_to_backend_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<StorageBackendRecord> {
    Ok(StorageBackendRecord {
        backend_id: row.get(0)?,
        backend_type: row.get(1)?,
        config_json: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}
