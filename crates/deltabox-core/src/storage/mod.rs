use anyhow::Result;

pub mod local;
pub mod s3;

pub trait StorageBackend {
    fn backend_id(&self) -> &str;
    fn put_chunk(&self, chunk_id: &str, data: &[u8]) -> Result<()>;
    fn get_chunk(&self, chunk_id: &str) -> Result<Vec<u8>>;
    fn has_chunk(&self, chunk_id: &str) -> Result<bool>;
    fn delete_chunk(&self, chunk_id: &str) -> Result<()>;
}
