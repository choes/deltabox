use anyhow::Result;
use rusqlite::{params, Connection};

use crate::files::row_to_file_record;
use crate::util::fts_phrase_query;
use crate::{FileRecord, SearchMatch, SearchResult, Vault};

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

    pub fn search_files_detailed(
        &self,
        query: &str,
        include_trashed: bool,
    ) -> Result<Vec<SearchResult>> {
        let files = self.search_files(query, include_trashed)?;
        let conn = self.open_db()?;
        let mut results = Vec::new();

        for file in files {
            let mut matches = Vec::new();
            if contains_case_insensitive(&file.name, query) {
                matches.push(SearchMatch {
                    match_kind: "name".to_owned(),
                    source: None,
                    text: Some(file.name.clone()),
                    page: None,
                    line_start: None,
                    line_end: None,
                    score: None,
                });
            }
            if contains_case_insensitive(&file.logical_path, query) {
                matches.push(SearchMatch {
                    match_kind: "path".to_owned(),
                    source: None,
                    text: Some(file.logical_path.clone()),
                    page: None,
                    line_start: None,
                    line_end: None,
                    score: None,
                });
            }
            matches.extend(tag_matches(&conn, &file.file_id, query)?);
            matches.extend(text_segment_matches(&conn, &file.file_id, query, 5)?);
            results.push(SearchResult { file, matches });
        }

        Ok(results)
    }
}

fn tag_matches(conn: &Connection, file_id: &str, query: &str) -> Result<Vec<SearchMatch>> {
    let needle = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT t.name
         FROM tags t
         JOIN file_tags ft ON ft.tag_id = t.tag_id
         WHERE ft.file_id = ?1 AND t.name LIKE ?2
         ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![file_id, needle], |row| {
        let name: String = row.get(0)?;
        Ok(SearchMatch {
            match_kind: "tag".to_owned(),
            source: None,
            text: Some(name),
            page: None,
            line_start: None,
            line_end: None,
            score: None,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn text_segment_matches(
    conn: &Connection,
    file_id: &str,
    query: &str,
    limit: u64,
) -> Result<Vec<SearchMatch>> {
    let needle = format!("%{query}%");
    let fts_query = fts_phrase_query(query);
    let mut stmt = conn.prepare(
        "SELECT DISTINCT ts.source, ts.text, ts.page, ts.line_start, ts.line_end
         FROM text_segments ts
         WHERE ts.file_id = ?1
           AND (
                ts.text LIKE ?2
                OR ts.segment_id IN (
                    SELECT segment_id FROM text_segments_fts
                    WHERE file_id = ?1 AND text_segments_fts MATCH ?3
                )
           )
         ORDER BY ts.segment_index
        LIMIT ?4",
    )?;
    let rows = stmt.query_map(params![file_id, needle, fts_query, limit], |row| {
        Ok(SearchMatch {
            match_kind: "text".to_owned(),
            source: Some(row.get(0)?),
            text: Some(snippet(row.get::<_, String>(1)?.as_str(), query, 240)),
            page: row.get(2)?,
            line_start: row.get(3)?,
            line_end: row.get(4)?,
            score: None,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn contains_case_insensitive(value: &str, query: &str) -> bool {
    value.to_lowercase().contains(&query.to_lowercase())
}

fn snippet(text: &str, query: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }

    let lower = normalized.to_lowercase();
    let query_lower = query.to_lowercase();
    let byte_start = lower.find(&query_lower).unwrap_or(0);
    let char_start = normalized[..byte_start].chars().count();
    let half = max_chars / 2;
    let start = char_start.saturating_sub(half);
    let end = start + max_chars;
    let mut output = normalized
        .chars()
        .skip(start)
        .take(max_chars)
        .collect::<String>();
    if start > 0 {
        output.insert_str(0, "...");
    }
    if normalized.chars().count() > end {
        output.push_str("...");
    }
    output
}
