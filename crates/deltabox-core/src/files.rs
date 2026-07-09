use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rusqlite::params;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::manifest::{ChunkRef, FileManifest, FileStatus, LocationRef};
use crate::storage::{local::LocalStorage, StorageBackend};
use crate::util::{format_system_time, guess_mime, now_rfc3339, sha256_hex};
use crate::{AddOptions, FileRecord, TrashRecord, Vault, DEFAULT_CHUNK_SIZE};

impl Vault {
    pub fn add_file(&self, options: AddOptions) -> Result<FileManifest> {
        let source = options.source;
        let metadata = fs::metadata(&source)
            .with_context(|| format!("failed to read source metadata: {}", source.display()))?;
        if !metadata.is_file() {
            return Err(anyhow!(
                "source is not a regular file: {}",
                source.display()
            ));
        }

        let name = source
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or_else(|| anyhow!("source file has no valid UTF-8 name"))?
            .to_owned();
        let logical_path = options.logical_path.unwrap_or_else(|| format!("/{}", name));
        let now = now_rfc3339();
        let file_id = Uuid::new_v4().to_string();
        let backend = LocalStorage::new("local", self.chunk_dir.clone());

        let mut file = fs::File::open(&source)?;
        let mut chunks = Vec::new();
        let mut offset = 0_u64;
        let mut content_hasher = Sha256::new();
        let mut buffer = vec![0_u8; DEFAULT_CHUNK_SIZE];

        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            let data = &buffer[..read];
            content_hasher.update(data);
            let chunk_hash = sha256_hex(data);
            let chunk_id = format!("sha256:{chunk_hash}");
            backend.put_chunk(&chunk_id, data)?;
            chunks.push(ChunkRef {
                chunk_id: chunk_id.clone(),
                hash: chunk_id,
                offset,
                size: read as u64,
                locations: vec![LocationRef {
                    backend_id: backend.backend_id().to_owned(),
                    object_key: LocalStorage::object_key_for_hash(&chunk_hash),
                    status: "available".to_owned(),
                }],
            });
            offset += read as u64;
        }

        let content_hash = format!("sha256:{}", hex::encode(content_hasher.finalize()));
        let manifest = FileManifest {
            file_id,
            name,
            logical_path,
            mime: guess_mime(&source),
            size: metadata.len(),
            content_hash,
            version: 1,
            status: FileStatus::Active,
            created_at: metadata
                .created()
                .ok()
                .map(format_system_time)
                .unwrap_or_else(|| now.clone()),
            modified_at: metadata
                .modified()
                .ok()
                .map(format_system_time)
                .unwrap_or_else(|| now.clone()),
            imported_at: now,
            trashed_at: None,
            chunks,
            tags: Vec::new(),
            replica_policy: None,
        };

        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event("file.created", Some(&manifest.file_id), None)?;
        self.index_text_file_from_path(&manifest, &source)?;
        Ok(manifest)
    }

    pub fn list_files(&self, include_trashed: bool) -> Result<Vec<FileRecord>> {
        let conn = self.open_db()?;
        let sql = if include_trashed {
            "SELECT file_id, name, logical_path, size, content_hash, status, imported_at, trashed_at FROM files ORDER BY imported_at DESC"
        } else {
            "SELECT file_id, name, logical_path, size, content_hash, status, imported_at, trashed_at FROM files WHERE status = 'active' ORDER BY imported_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], row_to_file_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn restore_file(&self, file_id: &str, output: impl AsRef<Path>) -> Result<PathBuf> {
        let manifest = self.get_manifest(file_id)?;
        if manifest.status == FileStatus::Purged {
            return Err(anyhow!("file has been permanently deleted: {file_id}"));
        }
        let output = output.as_ref();
        let destination = if output.is_dir() {
            output.join(&manifest.name)
        } else {
            output.to_path_buf()
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out = fs::File::create(&destination)?;
        for chunk in self.read_file_chunks(&manifest)? {
            out.write_all(&chunk)?;
        }
        Ok(destination)
    }

    pub fn move_to_trash(&self, file_id: &str) -> Result<FileManifest> {
        let mut manifest = self.get_manifest(file_id)?;
        if manifest.status == FileStatus::Purged {
            return Err(anyhow!("file is already purged: {file_id}"));
        }
        manifest.status = FileStatus::Trashed;
        manifest.trashed_at = Some(now_rfc3339());
        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event("file.trashed", Some(file_id), Some(&manifest.logical_path))?;
        Ok(manifest)
    }

    pub fn restore_from_trash(&self, file_id: &str) -> Result<FileManifest> {
        let mut manifest = self.get_manifest(file_id)?;
        if manifest.status != FileStatus::Trashed {
            return Err(anyhow!("file is not in trash: {file_id}"));
        }
        manifest.status = FileStatus::Active;
        manifest.trashed_at = None;
        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event("file.restored", Some(file_id), Some(&manifest.logical_path))?;
        Ok(manifest)
    }

    pub fn list_trash(&self) -> Result<Vec<TrashRecord>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT file_id, name, logical_path, size, trashed_at FROM files WHERE status = 'trashed' ORDER BY trashed_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(TrashRecord {
                file_id: row.get(0)?,
                name: row.get(1)?,
                previous_path: row.get(2)?,
                size: row.get(3)?,
                trashed_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn purge_file(&self, file_id: &str) -> Result<()> {
        let mut manifest = self.get_manifest(file_id)?;
        manifest.status = FileStatus::Purged;
        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event("file.purged", Some(file_id), Some(&manifest.logical_path))?;
        Ok(())
    }

    pub fn gc_chunks(&self) -> Result<usize> {
        let referenced = self.referenced_local_object_keys()?;
        let mut removed = 0_usize;
        let backend = LocalStorage::new("local", self.chunk_dir.clone());
        for object_key in backend.list_object_keys()? {
            if !referenced.contains(&object_key) {
                backend.delete_object_key(&object_key)?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub(crate) fn upsert_file_record(&self, manifest: &FileManifest) -> Result<()> {
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO files (file_id, name, logical_path, mime, size, content_hash, version, status, imported_at, trashed_at, manifest_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(file_id) DO UPDATE SET
               name = excluded.name,
               logical_path = excluded.logical_path,
               mime = excluded.mime,
               size = excluded.size,
               content_hash = excluded.content_hash,
               version = excluded.version,
               status = excluded.status,
               imported_at = excluded.imported_at,
               trashed_at = excluded.trashed_at,
               manifest_path = excluded.manifest_path",
            params![
                manifest.file_id,
                manifest.name,
                manifest.logical_path,
                manifest.mime,
                manifest.size,
                manifest.content_hash,
                manifest.version,
                manifest.status.as_str(),
                manifest.imported_at,
                manifest.trashed_at,
                self.manifest_path(&manifest.file_id).to_string_lossy(),
            ],
        )?;

        for chunk in &manifest.chunks {
            for location in &chunk.locations {
                conn.execute(
                    "INSERT OR IGNORE INTO chunks (chunk_id, hash, size, backend_id, object_key)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        chunk.chunk_id,
                        chunk.hash,
                        chunk.size,
                        location.backend_id,
                        location.object_key,
                    ],
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn read_file_bytes(&self, manifest: &FileManifest) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(manifest.size as usize);
        for chunk in self.read_file_chunks(manifest)? {
            bytes.extend(chunk);
        }
        Ok(bytes)
    }

    pub(crate) fn ensure_file_exists(&self, file_id: &str) -> Result<()> {
        let conn = self.open_db()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM files WHERE file_id = ?1 AND status != 'purged')",
            params![file_id],
            |row| row.get(0),
        )?;
        if exists {
            Ok(())
        } else {
            Err(anyhow!("file not found: {file_id}"))
        }
    }

    fn read_file_chunks(&self, manifest: &FileManifest) -> Result<Vec<Vec<u8>>> {
        if manifest.status == FileStatus::Purged {
            return Err(anyhow!(
                "file has been permanently deleted: {}",
                manifest.file_id
            ));
        }
        let mut chunks = manifest.chunks.clone();
        chunks.sort_by_key(|chunk| chunk.offset);
        chunks
            .into_iter()
            .map(|chunk| self.read_chunk_from_any_location(&chunk.chunk_id, manifest))
            .collect()
    }

    fn referenced_local_object_keys(&self) -> Result<HashSet<String>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare("SELECT manifest_path FROM files WHERE status != 'purged'")?;
        let manifest_paths = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut referenced = HashSet::new();
        for manifest_path in manifest_paths {
            let data = fs::read_to_string(manifest_path)?;
            let manifest: FileManifest = serde_json::from_str(&data)?;
            for chunk in manifest.chunks {
                for location in chunk.locations {
                    if location.backend_id == "local" {
                        referenced.insert(location.object_key);
                    }
                }
            }
        }
        Ok(referenced)
    }
}

pub(crate) fn row_to_file_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
    let status: String = row.get(5)?;
    Ok(FileRecord {
        file_id: row.get(0)?,
        name: row.get(1)?,
        logical_path: row.get(2)?,
        size: row.get(3)?,
        content_hash: row.get(4)?,
        status: FileStatus::from_db(&status),
        imported_at: row.get(6)?,
        trashed_at: row.get(7)?,
    })
}
