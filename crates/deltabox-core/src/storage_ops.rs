use anyhow::{anyhow, Result};

use crate::manifest::{FileManifest, LocationRef, ReplicaPolicy, ReplicaPolicyMode};
use crate::{ChunkLocationRecord, StorageVerifyRecord, Vault};

impl Vault {
    pub fn copy_file_to_backend(
        &self,
        file_id: &str,
        target_backend_id: &str,
    ) -> Result<FileManifest> {
        self.ensure_file_exists(file_id)?;
        let source_manifest = self.get_manifest(file_id)?;
        let mut manifest = source_manifest.clone();
        let target = self.storage_backend_by_id(target_backend_id)?;

        for chunk in &mut manifest.chunks {
            if chunk
                .locations
                .iter()
                .any(|location| location.backend_id == target_backend_id)
            {
                continue;
            }
            let source_data =
                self.read_chunk_from_any_location(&chunk.chunk_id, &source_manifest)?;
            target.put_chunk(&chunk.chunk_id, &source_data)?;
            let object_key =
                self.object_key_for_backend_chunk(target_backend_id, &chunk.chunk_id)?;
            chunk.locations.push(LocationRef {
                backend_id: target_backend_id.to_owned(),
                object_key,
                status: "available".to_owned(),
            });
        }

        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event(
            "storage.copied",
            Some(&manifest.file_id),
            Some(target_backend_id),
        )?;
        Ok(manifest)
    }

    pub fn file_locations(&self, file_id: &str) -> Result<Vec<ChunkLocationRecord>> {
        self.ensure_file_exists(file_id)?;
        let manifest = self.get_manifest(file_id)?;
        let mut locations = Vec::new();
        for chunk in manifest.chunks {
            for location in chunk.locations {
                locations.push(ChunkLocationRecord {
                    chunk_id: chunk.chunk_id.clone(),
                    backend_id: location.backend_id,
                    object_key: location.object_key,
                    status: location.status,
                });
            }
        }
        Ok(locations)
    }

    pub fn verify_file_locations(&self, file_id: &str) -> Result<Vec<StorageVerifyRecord>> {
        self.ensure_file_exists(file_id)?;
        let manifest = self.get_manifest(file_id)?;
        let mut records = Vec::new();
        for chunk in &manifest.chunks {
            for location in &chunk.locations {
                let result = self
                    .storage_backend_by_id(&location.backend_id)
                    .and_then(|backend| backend.has_chunk(&chunk.chunk_id));
                match result {
                    Ok(true) => records.push(StorageVerifyRecord {
                        chunk_id: chunk.chunk_id.clone(),
                        backend_id: location.backend_id.clone(),
                        ok: true,
                        message: "ok".to_owned(),
                    }),
                    Ok(false) => records.push(StorageVerifyRecord {
                        chunk_id: chunk.chunk_id.clone(),
                        backend_id: location.backend_id.clone(),
                        ok: false,
                        message: "missing".to_owned(),
                    }),
                    Err(error) => records.push(StorageVerifyRecord {
                        chunk_id: chunk.chunk_id.clone(),
                        backend_id: location.backend_id.clone(),
                        ok: false,
                        message: error.to_string(),
                    }),
                }
            }
        }
        Ok(records)
    }

    pub fn remove_file_location(
        &self,
        file_id: &str,
        backend_id: &str,
        force: bool,
    ) -> Result<FileManifest> {
        self.ensure_file_exists(file_id)?;
        let mut manifest = self.get_manifest(file_id)?;

        for chunk in &manifest.chunks {
            let available_locations = chunk
                .locations
                .iter()
                .filter(|location| location.status == "available")
                .count();
            let removing = chunk.locations.iter().any(|location| {
                location.backend_id == backend_id && location.status == "available"
            });
            if removing && available_locations <= 1 && !force {
                return Err(anyhow!(
                    "refusing to remove the only available location for chunk {}",
                    chunk.chunk_id
                ));
            }
        }

        for chunk in &mut manifest.chunks {
            if let Ok(backend) = self.storage_backend_by_id(backend_id) {
                let _ = backend.delete_chunk(&chunk.chunk_id);
            }
            chunk
                .locations
                .retain(|location| location.backend_id != backend_id);
        }

        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event(
            "storage.location_removed",
            Some(&manifest.file_id),
            Some(backend_id),
        )?;
        Ok(manifest)
    }

    pub fn move_file_to_backend(
        &self,
        file_id: &str,
        target_backend_id: &str,
    ) -> Result<FileManifest> {
        let copied = self.copy_file_to_backend(file_id, target_backend_id)?;
        let source_backend = copied
            .chunks
            .iter()
            .flat_map(|chunk| chunk.locations.iter())
            .find(|location| location.backend_id != target_backend_id)
            .map(|location| location.backend_id.clone());

        let Some(source_backend) = source_backend else {
            return Ok(copied);
        };

        let verify = self.verify_file_locations(file_id)?;
        let target_ok = verify
            .iter()
            .any(|record| record.backend_id == target_backend_id && record.ok);
        if !target_ok {
            return Err(anyhow!(
                "target backend was not verified after copy: {target_backend_id}"
            ));
        }

        self.remove_file_location(file_id, &source_backend, false)
    }

    pub fn set_replica_policy(
        &self,
        file_id: &str,
        mode: ReplicaPolicyMode,
        min_full_copies: u64,
        preferred_backends: Vec<String>,
        cache_backends: Vec<String>,
        local_cache_ttl_days: Option<u64>,
    ) -> Result<FileManifest> {
        self.ensure_file_exists(file_id)?;
        let mut manifest = self.get_manifest(file_id)?;
        manifest.replica_policy = Some(ReplicaPolicy {
            mode,
            min_full_copies,
            preferred_backends,
            cache_backends,
            local_cache_ttl_days,
        });
        self.save_manifest(&manifest)?;
        self.upsert_file_record(&manifest)?;
        self.record_event("replica_policy.updated", Some(file_id), None)?;
        Ok(manifest)
    }

    fn read_chunk_from_any_location(
        &self,
        chunk_id: &str,
        manifest: &FileManifest,
    ) -> Result<Vec<u8>> {
        let chunk = manifest
            .chunks
            .iter()
            .find(|chunk| chunk.chunk_id == chunk_id)
            .ok_or_else(|| anyhow!("chunk not found in manifest: {chunk_id}"))?;
        for location in &chunk.locations {
            if location.status != "available" {
                continue;
            }
            if let Ok(backend) = self.storage_backend_by_id(&location.backend_id) {
                if backend.has_chunk(chunk_id)? {
                    return backend.get_chunk(chunk_id);
                }
            }
        }
        Err(anyhow!("no available location for chunk: {chunk_id}"))
    }
}
