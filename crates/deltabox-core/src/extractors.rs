use anyhow::Result;

use crate::manifest::FileManifest;
use crate::util::{split_text_segments, TextSegmentDraft};

#[derive(Debug, Clone)]
pub(crate) struct ExtractedTextSegment {
    pub(crate) source: String,
    pub(crate) task_key: String,
    pub(crate) segment_index: u64,
    pub(crate) text: String,
    pub(crate) page: Option<u64>,
    pub(crate) line_start: Option<u64>,
    pub(crate) line_end: Option<u64>,
    pub(crate) char_start: Option<u64>,
    pub(crate) char_end: Option<u64>,
    pub(crate) start_ms: Option<u64>,
    pub(crate) end_ms: Option<u64>,
    pub(crate) confidence: f64,
}

pub(crate) trait TextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool;
    fn extract(&self, manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractedTextSegment>>;
}

pub(crate) fn extractor_for_manifest(manifest: &FileManifest) -> Option<Box<dyn TextExtractor>> {
    let extractors: Vec<Box<dyn TextExtractor>> =
        vec![Box::new(Utf8TextExtractor), Box::new(PdfTextExtractor)];
    extractors
        .into_iter()
        .find(|extractor| extractor.supports(manifest))
}

pub(crate) fn is_text_extractable(manifest: &FileManifest) -> bool {
    extractor_for_manifest(manifest).is_some()
}

struct Utf8TextExtractor;

impl TextExtractor for Utf8TextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool {
        matches!(
            manifest.mime.as_str(),
            "text/plain" | "text/markdown" | "text/csv" | "application/json"
        )
    }

    fn extract(&self, manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractedTextSegment>> {
        let text = std::str::from_utf8(bytes)?;
        let source = text_source_for_mime(&manifest.mime).to_owned();
        Ok(split_text_segments(text, 100)
            .into_iter()
            .enumerate()
            .map(|(segment_index, segment)| {
                segment_from_text(source.clone(), segment_index, segment)
            })
            .collect())
    }
}

struct PdfTextExtractor;

impl TextExtractor for PdfTextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool {
        manifest.mime == "application/pdf"
    }

    fn extract(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractedTextSegment>> {
        let pages = pdf_extract::extract_text_from_mem_by_pages(bytes)?;
        let mut segments = Vec::new();

        for (page_index, page_text) in pages.into_iter().enumerate() {
            let page = page_index as u64 + 1;
            for segment in split_text_segments(&page_text, 100) {
                let segment_index = segments.len() as u64;
                segments.push(ExtractedTextSegment {
                    source: "pdf_text".to_owned(),
                    task_key: format!("page:{page}:chunk:{segment_index}"),
                    segment_index,
                    text: segment.text,
                    page: Some(page),
                    line_start: Some(segment.line_start),
                    line_end: Some(segment.line_end),
                    char_start: Some(segment.char_start),
                    char_end: Some(segment.char_end),
                    start_ms: None,
                    end_ms: None,
                    confidence: 1.0,
                });
            }
        }

        Ok(segments)
    }
}

fn segment_from_text(
    source: String,
    segment_index: usize,
    segment: TextSegmentDraft,
) -> ExtractedTextSegment {
    ExtractedTextSegment {
        source,
        task_key: format!("text_chunk:{segment_index}"),
        segment_index: segment_index as u64,
        text: segment.text,
        page: None,
        line_start: Some(segment.line_start),
        line_end: Some(segment.line_end),
        char_start: Some(segment.char_start),
        char_end: Some(segment.char_end),
        start_ms: None,
        end_ms: None,
        confidence: 1.0,
    }
}

fn text_source_for_mime(mime: &str) -> &'static str {
    match mime {
        "text/markdown" => "markdown",
        "text/csv" => "csv",
        "application/json" => "json",
        _ => "plain_text",
    }
}
