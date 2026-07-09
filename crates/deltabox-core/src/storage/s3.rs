use std::sync::Arc;

use anyhow::{anyhow, Result};
use object_store::aws::AmazonS3Builder;
use object_store::path::Path;
use object_store::{ObjectStore, PutPayload};
use tokio::runtime::Runtime;

use crate::storage::StorageBackend;

#[derive(Debug, Clone)]
pub struct S3StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub prefix: Option<String>,
    pub allow_http: bool,
    pub path_style: bool,
}

pub struct S3Storage {
    backend_id: String,
    prefix: Option<String>,
    store: Arc<dyn ObjectStore>,
    runtime: Runtime,
}

impl S3Storage {
    pub fn new(backend_id: impl Into<String>, config: S3StorageConfig) -> Result<Self> {
        let mut builder = AmazonS3Builder::new()
            .with_endpoint(config.endpoint)
            .with_bucket_name(config.bucket)
            .with_region(config.region)
            .with_access_key_id(config.access_key)
            .with_secret_access_key(config.secret_key);

        if config.allow_http {
            builder = builder.with_allow_http(true);
        }
        if config.path_style {
            builder = builder.with_virtual_hosted_style_request(false);
        }

        let store = builder.build()?;
        Ok(Self {
            backend_id: backend_id.into(),
            prefix: config.prefix,
            store: Arc::new(store),
            runtime: Runtime::new()?,
        })
    }

    pub fn object_key_for_hash(hash: &str) -> String {
        let prefix = hash.get(0..2).unwrap_or("00");
        let rest = hash.get(2..).unwrap_or(hash);
        format!("{prefix}/{rest}")
    }

    fn object_key_for_chunk_id(&self, chunk_id: &str) -> Result<String> {
        let hash = chunk_id
            .strip_prefix("sha256:")
            .ok_or_else(|| anyhow!("unsupported chunk id: {chunk_id}"))?;
        let key = Self::object_key_for_hash(hash);
        Ok(
            match self.prefix.as_deref().filter(|value| !value.is_empty()) {
                Some(prefix) => format!("{}/{}", prefix.trim_matches('/'), key),
                None => key,
            },
        )
    }

    fn path_for_chunk_id(&self, chunk_id: &str) -> Result<Path> {
        Ok(Path::from(self.object_key_for_chunk_id(chunk_id)?))
    }
}

impl StorageBackend for S3Storage {
    fn backend_id(&self) -> &str {
        &self.backend_id
    }

    fn put_chunk(&self, chunk_id: &str, data: &[u8]) -> Result<()> {
        let path = self.path_for_chunk_id(chunk_id)?;
        let payload = PutPayload::from(data.to_vec());
        self.runtime
            .block_on(async { self.store.put(&path, payload).await })?;
        Ok(())
    }

    fn get_chunk(&self, chunk_id: &str) -> Result<Vec<u8>> {
        let path = self.path_for_chunk_id(chunk_id)?;
        let bytes = self
            .runtime
            .block_on(async { self.store.get(&path).await?.bytes().await })?;
        Ok(bytes.to_vec())
    }

    fn has_chunk(&self, chunk_id: &str) -> Result<bool> {
        let path = self.path_for_chunk_id(chunk_id)?;
        let result = self
            .runtime
            .block_on(async { self.store.head(&path).await });
        match result {
            Ok(_) => Ok(true),
            Err(object_store::Error::NotFound { .. }) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn delete_chunk(&self, chunk_id: &str) -> Result<()> {
        let path = self.path_for_chunk_id(chunk_id)?;
        let result = self
            .runtime
            .block_on(async { self.store.delete(&path).await });
        match result {
            Ok(_) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}
