use std::io::{Cursor, Read};

use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use zip::result::ZipError;
use zip::ZipArchive;

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
    let extractors: Vec<Box<dyn TextExtractor>> = vec![
        Box::new(Utf8TextExtractor),
        Box::new(PdfTextExtractor),
        Box::new(DocxTextExtractor),
        Box::new(XlsxTextExtractor),
        Box::new(PptxTextExtractor),
    ];
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

    fn plan_tasks(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        let text = std::str::from_utf8(bytes)?;
        Ok(split_text_segments(text, 100)
            .into_iter()
            .enumerate()
            .map(|(segment_index, _)| ExtractionTask {
                task_key: format!("text:chunk:{segment_index}"),
            })
            .collect())
    }

    fn extract_task(
        &self,
        manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        let task_index = task
            .task_key
            .strip_prefix("text:chunk:")
            .ok_or_else(|| {
                anyhow::anyhow!("unsupported UTF-8 text extraction task: {}", task.task_key)
            })?
            .parse::<usize>()?;
        let text = std::str::from_utf8(bytes)?;
        let source = text_source_for_mime(&manifest.mime).to_owned();
        let segment = split_text_segments(text, 100)
            .into_iter()
            .nth(task_index)
            .ok_or_else(|| anyhow::anyhow!("text chunk task out of range: {}", task.task_key))?;
        Ok(vec![segment_from_text(
            source,
            task.task_key.clone(),
            task_index,
            segment,
        )])
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

struct DocxTextExtractor;

impl TextExtractor for DocxTextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool {
        manifest.mime == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    }

    fn plan_tasks(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        let text = extract_docx_document_text(bytes)?;
        Ok(split_text_segments(&text, 100)
            .into_iter()
            .enumerate()
            .map(|(segment_index, _)| ExtractionTask {
                task_key: format!("docx:chunk:{segment_index}"),
            })
            .collect())
    }

    fn extract_task(
        &self,
        _manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        let task_index = task
            .task_key
            .strip_prefix("docx:chunk:")
            .ok_or_else(|| anyhow::anyhow!("unsupported DOCX extraction task: {}", task.task_key))?
            .parse::<usize>()?;
        let text = extract_docx_document_text(bytes)?;
        let segment = split_text_segments(&text, 100)
            .into_iter()
            .nth(task_index)
            .ok_or_else(|| anyhow::anyhow!("DOCX chunk task out of range: {}", task.task_key))?;
        Ok(vec![segment_from_text(
            "docx_text".to_owned(),
            task.task_key.clone(),
            task_index,
            segment,
        )])
    }
}

struct XlsxTextExtractor;

impl TextExtractor for XlsxTextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool {
        manifest.mime == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    }

    fn plan_tasks(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        let text = extract_xlsx_workbook_text(bytes)?;
        Ok(split_text_segments(&text, 100)
            .into_iter()
            .enumerate()
            .map(|(segment_index, _)| ExtractionTask {
                task_key: format!("xlsx:chunk:{segment_index}"),
            })
            .collect())
    }

    fn extract_task(
        &self,
        _manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        let task_index = task
            .task_key
            .strip_prefix("xlsx:chunk:")
            .ok_or_else(|| anyhow::anyhow!("unsupported XLSX extraction task: {}", task.task_key))?
            .parse::<usize>()?;
        let text = extract_xlsx_workbook_text(bytes)?;
        let segment = split_text_segments(&text, 100)
            .into_iter()
            .nth(task_index)
            .ok_or_else(|| anyhow::anyhow!("XLSX chunk task out of range: {}", task.task_key))?;
        Ok(vec![segment_from_text(
            "xlsx_text".to_owned(),
            task.task_key.clone(),
            task_index,
            segment,
        )])
    }
}

struct PptxTextExtractor;

impl TextExtractor for PptxTextExtractor {
    fn supports(&self, manifest: &FileManifest) -> bool {
        manifest.mime == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    }

    fn plan_tasks(&self, _manifest: &FileManifest, bytes: &[u8]) -> Result<Vec<ExtractionTask>> {
        let slides = pptx_slide_names(bytes)?;
        Ok(slides
            .into_iter()
            .enumerate()
            .map(|(index, _)| ExtractionTask {
                task_key: format!("pptx:slide:{}", index + 1),
            })
            .collect())
    }

    fn extract_task(
        &self,
        _manifest: &FileManifest,
        bytes: &[u8],
        task: &ExtractionTask,
    ) -> Result<Vec<ExtractedTextSegment>> {
        let slide = task
            .task_key
            .strip_prefix("pptx:slide:")
            .ok_or_else(|| anyhow::anyhow!("unsupported PPTX extraction task: {}", task.task_key))?
            .parse::<u64>()?;
        let slide_name = pptx_slide_names(bytes)?
            .get(slide.saturating_sub(1) as usize)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("PPTX slide task out of range: {}", task.task_key))?;
        let text = extract_pptx_slide_text(bytes, &slide_name)?;
        let mut segments = Vec::new();

        for segment in split_text_segments(&text, 100) {
            let segment_index = segments.len() as u64;
            segments.push(ExtractedTextSegment {
                source: "pptx_text".to_owned(),
                task_key: task.task_key.clone(),
                segment_index,
                text: segment.text,
                page: Some(slide),
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
    task_key: String,
    segment_index: usize,
    segment: TextSegmentDraft,
) -> ExtractedTextSegment {
    ExtractedTextSegment {
        source,
        task_key,
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

fn extract_docx_document_text(bytes: &[u8]) -> Result<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut document = String::new();
    match archive.by_name("word/document.xml") {
        Ok(mut file) => file.read_to_string(&mut document)?,
        Err(ZipError::FileNotFound) => return Ok(String::new()),
        Err(error) => return Err(error.into()),
    };

    let mut reader = Reader::from_str(&document);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut text = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"t" {
                    in_text = true;
                }
            }
            Event::End(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"t" {
                    in_text = false;
                } else if local == b"p" && !text.ends_with('\n') {
                    text.push('\n');
                }
            }
            Event::Text(event) if in_text => {
                text.push_str(&event.unescape()?);
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(text)
}

fn extract_xlsx_workbook_text(bytes: &[u8]) -> Result<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let shared_strings = read_xlsx_shared_strings(&mut archive)?;
    let worksheet_names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_owned())
        })
        .filter(|name| name.starts_with("xl/worksheets/") && name.ends_with(".xml"))
        .collect::<Vec<_>>();

    let mut text = String::new();
    for worksheet_name in worksheet_names {
        let mut worksheet = String::new();
        archive
            .by_name(&worksheet_name)?
            .read_to_string(&mut worksheet)?;
        let worksheet_text = parse_xlsx_worksheet_text(&worksheet, &shared_strings)?;
        if !worksheet_text.trim().is_empty() {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&worksheet_text);
        }
    }

    Ok(text)
}

fn read_xlsx_shared_strings<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
) -> Result<Vec<String>> {
    let Ok(mut file) = archive.by_name("xl/sharedStrings.xml") else {
        return Ok(Vec::new());
    };
    let mut xml = String::new();
    file.read_to_string(&mut xml)?;
    parse_xlsx_shared_strings(&xml)
}

fn parse_xlsx_shared_strings(xml: &str) -> Result<Vec<String>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut shared_strings = Vec::new();
    let mut current = String::new();
    let mut in_shared_item = false;
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"si" {
                    in_shared_item = true;
                    current.clear();
                } else if local == b"t" && in_shared_item {
                    in_text = true;
                }
            }
            Event::End(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"t" {
                    in_text = false;
                } else if local == b"si" {
                    in_shared_item = false;
                    shared_strings.push(current.clone());
                }
            }
            Event::Text(event) if in_text => {
                current.push_str(&event.unescape()?);
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(shared_strings)
}

fn parse_xlsx_worksheet_text(xml: &str, shared_strings: &[String]) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut text = String::new();
    let mut cell_type: Option<String> = None;
    let mut cell_value = String::new();
    let mut in_cell = false;
    let mut in_value = false;
    let mut in_inline_text = false;
    let mut row_has_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"c" {
                    in_cell = true;
                    cell_value.clear();
                    cell_type = None;
                    for attribute in event.attributes().with_checks(false) {
                        let attribute = attribute?;
                        if local_xml_name(attribute.key.as_ref()) == b"t" {
                            cell_type = Some(attribute.unescape_value()?.into_owned());
                        }
                    }
                } else if local == b"v" && in_cell {
                    in_value = true;
                } else if local == b"t" && in_cell {
                    in_inline_text = true;
                }
            }
            Event::End(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"v" {
                    in_value = false;
                } else if local == b"t" {
                    in_inline_text = false;
                } else if local == b"c" {
                    let rendered = match cell_type.as_deref() {
                        Some("s") => cell_value
                            .parse::<usize>()
                            .ok()
                            .and_then(|index| shared_strings.get(index))
                            .map(String::as_str)
                            .unwrap_or(""),
                        _ => cell_value.as_str(),
                    };
                    if !rendered.trim().is_empty() {
                        if row_has_text && !text.ends_with(' ') {
                            text.push(' ');
                        }
                        text.push_str(rendered);
                        row_has_text = true;
                    }
                    in_cell = false;
                    cell_value.clear();
                    cell_type = None;
                } else if local == b"row" && row_has_text {
                    text.push('\n');
                    row_has_text = false;
                }
            }
            Event::Text(event) if in_value || in_inline_text => {
                cell_value.push_str(&event.unescape()?);
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(text)
}

fn pptx_slide_names(bytes: &[u8]) -> Result<Vec<String>> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut slide_names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_owned())
        })
        .filter(|name| {
            name.starts_with("ppt/slides/slide")
                && name.ends_with(".xml")
                && slide_number_from_name(name).is_some()
        })
        .collect::<Vec<_>>();
    slide_names.sort_by_key(|name| slide_number_from_name(name).unwrap_or(u64::MAX));
    Ok(slide_names)
}

fn slide_number_from_name(name: &str) -> Option<u64> {
    name.strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?
        .parse()
        .ok()
}

fn extract_pptx_slide_text(bytes: &[u8], slide_name: &str) -> Result<String> {
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;
    let mut slide = String::new();
    archive.by_name(slide_name)?.read_to_string(&mut slide)?;
    parse_pptx_slide_text(&slide)
}

fn parse_pptx_slide_text(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut text = String::new();
    let mut in_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"t" {
                    in_text = true;
                }
            }
            Event::End(event) => {
                let name = event.name();
                let local = local_xml_name(name.as_ref());
                if local == b"t" {
                    in_text = false;
                    if !text.ends_with('\n') {
                        text.push('\n');
                    }
                }
            }
            Event::Text(event) if in_text => {
                text.push_str(&event.unescape()?);
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(text)
}

fn local_xml_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn text_source_for_mime(mime: &str) -> &'static str {
    match mime {
        "text/markdown" => "markdown",
        "text/csv" => "csv",
        "application/json" => "json",
        _ => "plain_text",
    }
}
