use anyhow::Result;

use crate::manifest::FileManifest;
use crate::util::{split_text_segments, TextSegmentDraft};

#[derive(Debug, Clone)]
pub(crate) struct ExtractionTask {
    pub(crate) task_key: String,
}

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
    fn plan_tasks(&self, manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>>;
    fn extract_task(
        &self,
        manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>>;
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

    fn plan_tasks(&self, _manifest: &FileManifest, _bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        Ok(vec![ExtractionTask {
            task_key: "text:full".to_owned(),
        }])
    }

    fn extract_task(
        &self,
        manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        if task.task_key != "text:full" {
            anyhow::bail!("unsupported UTF-8 text extraction task: {}", task.task_key);
        }
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

    fn plan_tasks(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        let pages = pdf_extract::extract_text_from_mem_by_pages(bytes)?;
        Ok((1..=pages.len())
            .map(|page| ExtractionTask {
                task_key: format!("pdf:page:{page}"),
            })
            .collect())
    }

    fn extract_task(
        &self,
        _manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        let page = task
            .task_key
            .strip_prefix("pdf:page:")
            .ok_or_else(|| anyhow::anyhow!("unsupported PDF extraction task: {}", task.task_key))?
            .parse::<u64>()?;
        let pages = pdf_extract::extract_text_from_mem_by_pages(bytes)?;
        let page_text = pages
            .get(page.saturating_sub(1) as usize)
            .ok_or_else(|| anyhow::anyhow!("PDF page task out of range: {}", task.task_key))?;
        let mut segments = Vec::new();

        for segment in split_text_segments(page_text, 100) {
            let segment_index = segments.len() as u64;
            segments.push(ExtractedTextSegment {
                source: "pdf_text".to_owned(),
                task_key: task.task_key.clone(),
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
