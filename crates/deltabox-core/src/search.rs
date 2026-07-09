use anyhow::Result;
use rusqlite::params;

use crate::files::row_to_file_record;
use crate::util::fts_phrase_query;
use crate::{FileRecord, Vault};

impl Vault {
    pub fn search_files(&self, query: &str, include_trashed: bool) -> Result<Vec<FileRecord>> {
        let needle = format!("%{query}%");
        let fts_query = fts_phrase_query(query);
        let conn = self.open_db()?;
        let sql = if include_trashed {
            "SELECT DISTINCT f.file_id, f.name, f.logical_path, f.size, f.content_hash, f.status, f.imported_at, f.trashed_at
             FROM files f
             LEFT JOIN file_tags ft ON ft.file_id = f.file_id
             LEFT JOIN tags t ON t.tag_id = ft.tag_id
             LEFT JOIN text_segments ts ON ts.file_id = f.file_id
             WHERE f.name LIKE ?1
                OR f.logical_path LIKE ?1
                OR t.name LIKE ?1
                OR ts.text LIKE ?1
                OR f.file_id IN (SELECT file_id FROM text_segments_fts WHERE text_segments_fts MATCH ?2)
             ORDER BY f.imported_at DESC"
        } else {
            "SELECT DISTINCT f.file_id, f.name, f.logical_path, f.size, f.content_hash, f.status, f.imported_at, f.trashed_at
             FROM files f
             LEFT JOIN file_tags ft ON ft.file_id = f.file_id
             LEFT JOIN tags t ON t.tag_id = ft.tag_id
             LEFT JOIN text_segments ts ON ts.file_id = f.file_id
             WHERE f.status = 'active'
               AND (
                    f.name LIKE ?1
                    OR f.logical_path LIKE ?1
                    OR t.name LIKE ?1
                    OR ts.text LIKE ?1
                    OR f.file_id IN (SELECT file_id FROM text_segments_fts WHERE text_segments_fts MATCH ?2)
               )
             ORDER BY f.imported_at DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![needle, fts_query], row_to_file_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}
