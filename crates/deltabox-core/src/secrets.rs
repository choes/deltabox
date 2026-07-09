use std::fs;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use rusqlite::params;

use crate::util::now_rfc3339;
use crate::Vault;

const VAULT_KEY_FILE: &str = "vault.key";

impl Vault {
    pub(crate) fn ensure_vault_key(&self) -> Result<()> {
        let path = self.vault_key_path();
        if path.exists() {
            return Ok(());
        }
        let mut key = [0_u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        fs::write(&path, hex::encode(key))
            .with_context(|| format!("failed to write vault key: {}", path.display()))?;
        Ok(())
    }

    pub(crate) fn set_backend_secret(
        &self,
        backend_id: &str,
        secret_name: &str,
        value: &str,
    ) -> Result<()> {
        let key = self.load_vault_key()?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let mut nonce = [0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce);
        let encrypted = cipher
            .encrypt(Nonce::from_slice(&nonce), value.as_bytes())
            .map_err(|_| anyhow!("failed to encrypt backend secret"))?;
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO backend_secrets (backend_id, secret_name, nonce_hex, encrypted_value_hex, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(backend_id, secret_name) DO UPDATE SET
                nonce_hex = excluded.nonce_hex,
                encrypted_value_hex = excluded.encrypted_value_hex,
                updated_at = excluded.updated_at",
            params![
                backend_id,
                secret_name,
                hex::encode(nonce),
                hex::encode(encrypted),
                now,
                now,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn get_backend_secret(&self, backend_id: &str, secret_name: &str) -> Result<String> {
        let conn = self.open_db()?;
        let (nonce_hex, encrypted_value_hex): (String, String) = conn.query_row(
            "SELECT nonce_hex, encrypted_value_hex
             FROM backend_secrets
             WHERE backend_id = ?1 AND secret_name = ?2",
            params![backend_id, secret_name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let key = self.load_vault_key()?;
        let nonce = hex::decode(nonce_hex)?;
        let encrypted = hex::decode(encrypted_value_hex)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), encrypted.as_ref())
            .map_err(|_| anyhow!("failed to decrypt backend secret"))?;
        String::from_utf8(plaintext).context("backend secret is not valid UTF-8")
    }

    fn load_vault_key(&self) -> Result<[u8; 32]> {
        self.ensure_vault_key()?;
        let key_hex = fs::read_to_string(self.vault_key_path())?;
        let bytes = hex::decode(key_hex.trim())?;
        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow!("vault key has invalid length"))?;
        Ok(key)
    }

    fn vault_key_path(&self) -> PathBuf {
        self.meta_dir.join(VAULT_KEY_FILE)
    }
}
