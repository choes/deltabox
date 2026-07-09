use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Active,
    Trashed,
    Purged,
    Incomplete,
}

impl FileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Trashed => "trashed",
            Self::Purged => "purged",
            Self::Incomplete => "incomplete",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "active" => Self::Active,
            "trashed" => Self::Trashed,
            "purged" => Self::Purged,
            "incomplete" => Self::Incomplete,
            _ => Self::Incomplete,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifest {
    pub file_id: String,
    pub name: String,
    pub logical_path: String,
    pub mime: String,
    pub size: u64,
    pub content_hash: String,
    pub version: u64,
    pub status: FileStatus,
    pub created_at: String,
    pub modified_at: String,
    pub imported_at: String,
    pub trashed_at: Option<String>,
    pub chunks: Vec<ChunkRef>,
    pub tags: Vec<TagRef>,
    #[serde(default)]
    pub replica_policy: Option<ReplicaPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkRef {
    pub chunk_id: String,
    pub hash: String,
    pub offset: u64,
    pub size: u64,
    pub locations: Vec<LocationRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRef {
    pub backend_id: String,
    pub object_key: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRef {
    pub name: String,
    pub tag_type: String,
    pub source: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaPolicy {
    pub mode: ReplicaPolicyMode,
    pub min_full_copies: u64,
    pub preferred_backends: Vec<String>,
    pub cache_backends: Vec<String>,
    pub local_cache_ttl_days: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplicaPolicyMode {
    SingleCopy,
    LocalPrimaryCloudBackup,
    CloudPrimaryLocalCache,
    MirrorTwoBackends,
    MetadataOnlyLocal,
    NoBackup,
}

impl ReplicaPolicyMode {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "single_copy" => Some(Self::SingleCopy),
            "local_primary_cloud_backup" => Some(Self::LocalPrimaryCloudBackup),
            "cloud_primary_local_cache" => Some(Self::CloudPrimaryLocalCache),
            "mirror_two_backends" => Some(Self::MirrorTwoBackends),
            "metadata_only_local" => Some(Self::MetadataOnlyLocal),
            "no_backup" => Some(Self::NoBackup),
            _ => None,
        }
    }
}
