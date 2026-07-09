use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;

use crate::manifest::FileManifest;
use crate::schema::SCHEMA;
use crate::{Vault, VAULT_DIR};

impl Vault {
    pub fn init(root: impl AsRef<Path>) -> Result<Self> {
        let vault = Self::at(root);
        fs::create_dir_all(&vault.meta_dir)?;
        fs::create_dir_all(&vault.manifest_dir)?;
        fs::create_dir_all(&vault.chunk_dir)?;
        vault.open_db()?.execute_batch(SCHEMA)?;
        vault.ensure_vault_key()?;
        vault.ensure_default_local_backend()?;
        Ok(vault)
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let vault = Self::at(root);
        if !vault.db_path.exists() {
            return Err(anyhow!(
                "no deltabox vault found at {}. Run `deltabox init` first",
                vault.root.display()
            ));
        }
        vault.open_db()?.execute_batch(SCHEMA)?;
        vault.ensure_vault_key()?;
        vault.ensure_default_local_backend()?;
        Ok(vault)
    }

    pub(crate) fn at(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let meta_dir = root.join(VAULT_DIR);
        let manifest_dir = meta_dir.join("manifests");
        let chunk_dir = meta_dir.join("chunks");
        let db_path = meta_dir.join("metadata.sqlite3");
        Self {
            root,
            meta_dir,
            manifest_dir,
            chunk_dir,
            db_path,
        }
    }

    pub(crate) fn open_db(&self) -> Result<Connection> {
        Connection::open(&self.db_path).with_context(|| {
            format!(
                "failed to open deltabox metadata database: {}",
                self.db_path.display()
            )
        })
    }

    pub(crate) fn manifest_path(&self, file_id: &str) -> PathBuf {
        self.manifest_dir.join(format!("{file_id}.json"))
    }

    pub(crate) fn save_manifest(&self, manifest: &FileManifest) -> Result<()> {
        fs::create_dir_all(&self.manifest_dir)?;
        let path = self.manifest_path(&manifest.file_id);
        let data = serde_json::to_vec_pretty(manifest)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn get_manifest(&self, file_id: &str) -> Result<FileManifest> {
        let path = self.manifest_path(file_id);
        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read manifest: {}", path.display()))?;
        serde_json::from_str(&data).context("failed to parse manifest")
    }
}
