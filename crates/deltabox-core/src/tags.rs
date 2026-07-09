use anyhow::{anyhow, Result};
use rusqlite::{params, OptionalExtension};
use uuid::Uuid;

use crate::manifest::TagRef;
use crate::util::{normalize_tag_name, now_rfc3339};
use crate::{TagRecord, Vault};

impl Vault {
    pub fn create_tag(&self, name: &str, tag_type: &str) -> Result<TagRecord> {
        let normalized = normalize_tag_name(name)?;
        if let Some(existing) = self.find_tag_by_name(&normalized)? {
            return Ok(existing);
        }

        let now = now_rfc3339();
        let tag = TagRecord {
            tag_id: Uuid::new_v4().to_string(),
            name: normalized,
            tag_type: tag_type.to_owned(),
            source: "user".to_owned(),
            created_at: now.clone(),
            updated_at: now,
        };
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO tags (tag_id, name, tag_type, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                tag.tag_id,
                tag.name,
                tag.tag_type,
                tag.source,
                tag.created_at,
                tag.updated_at,
            ],
        )?;
        self.record_event("tag.created", Some(&tag.tag_id), Some(&tag.name))?;
        Ok(tag)
    }

    pub fn list_tags(&self) -> Result<Vec<TagRecord>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT tag_id, name, tag_type, source, created_at, updated_at FROM tags ORDER BY name",
        )?;
        let rows = stmt.query_map([], row_to_tag_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn rename_tag(&self, old_name: &str, new_name: &str) -> Result<TagRecord> {
        let old_name = normalize_tag_name(old_name)?;
        let new_name = normalize_tag_name(new_name)?;
        let mut tag = self
            .find_tag_by_name(&old_name)?
            .ok_or_else(|| anyhow!("tag not found: {old_name}"))?;
        if let Some(existing) = self.find_tag_by_name(&new_name)? {
            if existing.tag_id != tag.tag_id {
                return Err(anyhow!("target tag already exists: {new_name}"));
            }
        }
        tag.name = new_name;
        tag.updated_at = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE tags SET name = ?1, updated_at = ?2 WHERE tag_id = ?3",
            params![tag.name, tag.updated_at, tag.tag_id],
        )?;
        self.refresh_manifests_for_tag(&tag.tag_id)?;
        self.record_event("tag.updated", Some(&tag.tag_id), Some(&tag.name))?;
        Ok(tag)
    }

    pub fn delete_tag(&self, name: &str) -> Result<()> {
        let name = normalize_tag_name(name)?;
        let tag = self
            .find_tag_by_name(&name)?
            .ok_or_else(|| anyhow!("tag not found: {name}"))?;
        let affected_files = self.file_ids_for_tag(&tag.tag_id)?;
        let conn = self.open_db()?;
        conn.execute(
            "DELETE FROM file_tags WHERE tag_id = ?1",
            params![tag.tag_id],
        )?;
        conn.execute("DELETE FROM tags WHERE tag_id = ?1", params![tag.tag_id])?;
        for file_id in affected_files {
            self.refresh_manifest_tags(&file_id)?;
        }
        self.record_event("tag.deleted", Some(&tag.tag_id), Some(&tag.name))?;
        Ok(())
    }

    pub fn attach_tag(&self, file_id: &str, tag_name: &str) -> Result<TagRecord> {
        self.ensure_file_exists(file_id)?;
        let tag = self.create_tag(tag_name, "generic")?;
        let conn = self.open_db()?;
        conn.execute(
            "INSERT OR IGNORE INTO file_tags (file_id, tag_id, source, confidence, created_at)
             VALUES (?1, ?2, 'user', 1.0, ?3)",
            params![file_id, tag.tag_id, now_rfc3339()],
        )?;
        self.refresh_manifest_tags(file_id)?;
        self.record_event("tag.attached", Some(file_id), Some(&tag.name))?;
        Ok(tag)
    }

    pub fn detach_tag(&self, file_id: &str, tag_name: &str) -> Result<()> {
        self.ensure_file_exists(file_id)?;
        let tag_name = normalize_tag_name(tag_name)?;
        let tag = self
            .find_tag_by_name(&tag_name)?
            .ok_or_else(|| anyhow!("tag not found: {tag_name}"))?;
        let conn = self.open_db()?;
        conn.execute(
            "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = ?2",
            params![file_id, tag.tag_id],
        )?;
        self.refresh_manifest_tags(file_id)?;
        self.record_event("tag.detached", Some(file_id), Some(&tag.name))?;
        Ok(())
    }

    pub fn tags_for_file(&self, file_id: &str) -> Result<Vec<TagRecord>> {
        self.ensure_file_exists(file_id)?;
        self.tags_for_file_unchecked(file_id)
    }

    fn find_tag_by_name(&self, name: &str) -> Result<Option<TagRecord>> {
        let conn = self.open_db()?;
        conn.query_row(
            "SELECT tag_id, name, tag_type, source, created_at, updated_at FROM tags WHERE name = ?1",
            params![name],
            row_to_tag_record,
        )
        .optional()
        .map_err(Into::into)
    }

    fn refresh_manifest_tags(&self, file_id: &str) -> Result<()> {
        let mut manifest = self.get_manifest(file_id)?;
        manifest.tags = self
            .tags_for_file_unchecked(file_id)?
            .into_iter()
            .map(|tag| TagRef {
                name: tag.name,
                tag_type: tag.tag_type,
                source: tag.source,
                confidence: 1.0,
            })
            .collect();
        self.save_manifest(&manifest)
    }

    fn refresh_manifests_for_tag(&self, tag_id: &str) -> Result<()> {
        for file_id in self.file_ids_for_tag(tag_id)? {
            self.refresh_manifest_tags(&file_id)?;
        }
        Ok(())
    }

    fn tags_for_file_unchecked(&self, file_id: &str) -> Result<Vec<TagRecord>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT t.tag_id, t.name, t.tag_type, t.source, t.created_at, t.updated_at
             FROM tags t
             INNER JOIN file_tags ft ON ft.tag_id = t.tag_id
             WHERE ft.file_id = ?1
             ORDER BY t.name",
        )?;
        let rows = stmt.query_map(params![file_id], row_to_tag_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn file_ids_for_tag(&self, tag_id: &str) -> Result<Vec<String>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare("SELECT file_id FROM file_tags WHERE tag_id = ?1")?;
        let rows = stmt.query_map(params![tag_id], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

fn row_to_tag_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<TagRecord> {
    Ok(TagRecord {
        tag_id: row.get(0)?,
        name: row.get(1)?,
        tag_type: row.get(2)?,
        source: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}
