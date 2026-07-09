use std::path::Path;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub(crate) struct TextSegmentDraft {
    pub(crate) text: String,
    pub(crate) line_start: u64,
    pub(crate) line_end: u64,
    pub(crate) char_start: u64,
    pub(crate) char_end: u64,
}

pub(crate) fn split_text_segments(text: &str, max_lines: usize) -> Vec<TextSegmentDraft> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut current = String::new();
    let mut segment_line_start = 1_u64;
    let mut segment_line_end = 1_u64;
    let mut segment_char_start = 0_u64;
    let mut current_char = 0_u64;

    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index as u64 + 1;
        if current.is_empty() {
            segment_line_start = line_number;
            segment_char_start = current_char;
        }

        current.push_str(line);
        current.push('\n');
        segment_line_end = line_number;
        current_char += line.len() as u64 + 1;

        if (segment_line_end - segment_line_start + 1) as usize >= max_lines {
            segments.push(TextSegmentDraft {
                text: std::mem::take(&mut current),
                line_start: segment_line_start,
                line_end: segment_line_end,
                char_start: segment_char_start,
                char_end: current_char,
            });
        }
    }

    if !current.is_empty() {
        segments.push(TextSegmentDraft {
            text: current,
            line_start: segment_line_start,
            line_end: segment_line_end,
            char_start: segment_char_start,
            char_end: current_char,
        });
    }

    segments
}

pub(crate) fn normalize_tag_name(name: &str) -> Result<String> {
    let normalized = name.trim().to_owned();
    if normalized.is_empty() {
        Err(anyhow!("tag name cannot be empty"))
    } else {
        Ok(normalized)
    }
}

pub(crate) fn fts_phrase_query(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

pub(crate) fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub(crate) fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

pub(crate) fn format_system_time(value: std::time::SystemTime) -> String {
    let datetime: OffsetDateTime = value.into();
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| now_rfc3339())
}

pub(crate) fn guess_mime(path: &Path) -> String {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
    {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        _ => "application/octet-stream",
    }
    .to_owned()
}
