pub mod manifest;
pub mod storage;

mod backends;
mod events;
mod extractors;
mod files;
mod index;
mod schema;
mod search;
mod secrets;
mod storage_ops;
mod tags;
mod util;
mod vault;

use std::path::PathBuf;

pub use index::IndexRunSummary;

use crate::manifest::FileStatus;

pub(crate) const DEFAULT_CHUNK_SIZE: usize = 1024 * 1024;
pub(crate) const VAULT_DIR: &str = ".deltabox";

#[derive(Debug, Clone)]
pub struct Vault {
    pub(crate) root: PathBuf,
    pub(crate) meta_dir: PathBuf,
    pub(crate) manifest_dir: PathBuf,
    pub(crate) chunk_dir: PathBuf,
    pub(crate) db_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AddOptions {
    pub source: PathBuf,
    pub logical_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FileRecord {
    pub file_id: String,
    pub name: String,
    pub logical_path: String,
    pub size: u64,
    pub content_hash: String,
    pub status: FileStatus,
    pub imported_at: String,
    pub trashed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrashRecord {
    pub file_id: String,
    pub name: String,
    pub previous_path: String,
    pub size: u64,
    pub trashed_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TagRecord {
    pub tag_id: String,
    pub name: String,
    pub tag_type: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexJobRecord {
    pub job_id: String,
    pub file_id: String,
    pub job_type: String,
    pub status: String,
    pub total_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub created_at: String,
    pub updated_at: String,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageBackendRecord {
    pub backend_id: String,
    pub backend_type: String,
    pub config_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ChunkLocationRecord {
    pub chunk_id: String,
    pub backend_id: String,
    pub object_key: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StorageVerifyRecord {
    pub chunk_id: String,
    pub backend_id: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub file: FileRecord,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchMatch {
    pub match_kind: String,
    pub source: Option<String>,
    pub text: Option<String>,
    pub page: Option<u64>,
    pub line_start: Option<u64>,
    pub line_end: Option<u64>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TextSegmentRecord {
    pub segment_id: String,
    pub file_id: String,
    pub source: String,
    pub task_key: String,
    pub segment_index: u64,
    pub text: String,
    pub page: Option<u64>,
    pub line_start: Option<u64>,
    pub line_end: Option<u64>,
    pub char_start: Option<u64>,
    pub char_end: Option<u64>,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
    pub confidence: f64,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Cursor, Write};

    use anyhow::Result;
    use uuid::Uuid;

    use super::*;

    #[test]
    fn add_restore_trash_restore_purge_and_gc() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        let output_dir = root.join("output");
        fs::create_dir_all(&input_dir)?;
        fs::create_dir_all(&output_dir)?;

        let input = input_dir.join("note.txt");
        fs::write(&input, b"hello deltabox\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input.clone(),
            logical_path: Some("/docs/note.txt".to_owned()),
        })?;

        let active = vault.list_files(false)?;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].logical_path, "/docs/note.txt");

        let restored = vault.restore_file(&manifest.file_id, output_dir.join("note.txt"))?;
        assert_eq!(fs::read(&input)?, fs::read(restored)?);

        vault.move_to_trash(&manifest.file_id)?;
        assert!(vault.list_files(false)?.is_empty());
        assert_eq!(vault.list_trash()?.len(), 1);

        vault.restore_from_trash(&manifest.file_id)?;
        assert_eq!(vault.list_files(false)?.len(), 1);

        vault.purge_file(&manifest.file_id)?;
        let removed = vault.gc_chunks()?;
        assert_eq!(removed, 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn tag_lifecycle_and_search() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("plan.txt");
        fs::write(&input, b"annual plan\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/work/plan.txt".to_owned()),
        })?;

        vault.create_tag("工作规划", "project")?;
        vault.attach_tag(&manifest.file_id, "工作规划")?;

        let tags = vault.tags_for_file(&manifest.file_id)?;
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "工作规划");

        let refreshed = vault.get_manifest(&manifest.file_id)?;
        assert_eq!(refreshed.tags.len(), 1);
        assert_eq!(refreshed.tags[0].name, "工作规划");

        let search = vault.search_files("工作规划", false)?;
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].file_id, manifest.file_id);

        vault.rename_tag("工作规划", "年度规划")?;
        assert!(vault.search_files("工作规划", false)?.is_empty());
        assert_eq!(vault.search_files("年度规划", false)?.len(), 1);
        assert_eq!(
            vault.get_manifest(&manifest.file_id)?.tags[0].name,
            "年度规划"
        );

        vault.detach_tag(&manifest.file_id, "年度规划")?;
        assert!(vault.tags_for_file(&manifest.file_id)?.is_empty());
        assert!(vault.search_files("年度规划", false)?.is_empty());

        vault.attach_tag(&manifest.file_id, "年度规划")?;
        vault.delete_tag("年度规划")?;
        assert!(vault.tags_for_file(&manifest.file_id)?.is_empty());
        assert!(vault.get_manifest(&manifest.file_id)?.tags.is_empty());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn full_text_index_searches_file_content() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("notes.txt");
        fs::write(
            &input,
            b"this filename does not reveal the keyword\nlibrary archive planning\n",
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/notes.txt".to_owned()),
        })?;

        let search = vault.search_files("library", false)?;
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].file_id, manifest.file_id);

        let jobs = vault.list_index_jobs()?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].status, "completed");
        assert_eq!(jobs[0].completed_tasks, 1);

        let rebuilt = vault.rebuild_text_index()?;
        assert_eq!(rebuilt.len(), 1);
        assert_eq!(vault.search_files("archive", false)?.len(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn index_worker_runs_enqueued_tasks() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("worker.txt");
        fs::write(&input, b"recoverable worker indexing\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/worker.txt".to_owned()),
        })?;

        vault.rebuild_text_index()?;
        let job = vault.enqueue_index_file(&manifest.file_id)?;
        assert_eq!(job.status, "pending");

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 1);
        assert_eq!(summary.failed, 0);

        let jobs = vault.list_index_jobs()?;
        assert!(jobs.iter().any(|job| job.status == "completed"));
        assert_eq!(vault.search_files("recoverable", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "plain_text");
        assert!(segments[0].text.contains("recoverable worker"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn utf8_text_index_tasks_resume_chunk_by_chunk() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("large.txt");
        let mut content = String::new();
        for line in 0..250 {
            content.push_str(&format!("line {line} common text\n"));
        }
        content.push_str("tail chunk searchable marker\n");
        fs::write(&input, content)?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/large.txt".to_owned()),
        })?;

        let job = vault.enqueue_index_file(&manifest.file_id)?;
        assert_eq!(job.total_tasks, 3);
        assert_eq!(job.completed_tasks, 0);

        let summary = vault.run_index_worker(1)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.completed_tasks, 1);
        assert_eq!(job.status, "pending");
        assert!(vault.search_files("tail chunk", false)?.is_empty());

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 2);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| {
                candidate.file_id == manifest.file_id && candidate.status == "completed"
            })
            .expect("completed index job");
        assert_eq!(job.total_tasks, 3);
        assert_eq!(job.completed_tasks, 3);
        assert_eq!(vault.search_files("tail chunk", false)?.len(), 1);

        let conn = vault.open_db()?;
        let task_segments: u64 = conn.query_row(
            "SELECT COUNT(DISTINCT task_key) FROM text_segments WHERE file_id = ?1",
            [&manifest.file_id],
            |row| row.get(0),
        )?;
        assert_eq!(task_segments, 3);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn index_job_pause_and_resume_controls_worker() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("pause.txt");
        fs::write(&input, b"pause resume worker indexing\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/pause.txt".to_owned()),
        })?;

        let job = vault.enqueue_index_file(&manifest.file_id)?;
        let paused = vault.pause_index_job(&job.job_id)?;
        assert_eq!(paused.status, "paused");

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 0);
        assert!(vault.search_files("resume", false)?.is_empty());

        let resumed = vault.resume_index_job(&job.job_id)?;
        assert_eq!(resumed.status, "pending");

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.status, "completed");
        assert_eq!(vault.search_files("resume", false)?.len(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn stale_running_tasks_requeue_only_after_timeout() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("stale.txt");
        fs::write(&input, b"stale timeout worker indexing\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/stale.txt".to_owned()),
        })?;
        let job = vault.enqueue_index_file(&manifest.file_id)?;

        let conn = vault.open_db()?;
        conn.execute(
            "UPDATE index_tasks SET status = 'running', updated_at = '9999-01-01T00:00:00Z' WHERE job_id = ?1",
            [&job.job_id],
        )?;
        conn.execute(
            "UPDATE index_jobs SET status = 'running', updated_at = '9999-01-01T00:00:00Z' WHERE job_id = ?1",
            [&job.job_id],
        )?;

        let summary = vault.run_index_worker_with_stale_timeout(10, 600)?;
        assert_eq!(summary.completed, 0);
        assert!(vault.search_files("timeout", false)?.is_empty());

        conn.execute(
            "UPDATE index_tasks SET status = 'running', updated_at = '1970-01-01T00:00:00Z' WHERE job_id = ?1",
            [&job.job_id],
        )?;
        conn.execute(
            "UPDATE index_jobs SET status = 'running', updated_at = '1970-01-01T00:00:00Z' WHERE job_id = ?1",
            [&job.job_id],
        )?;

        let summary = vault.run_index_worker_with_stale_timeout(10, 600)?;
        assert_eq!(summary.completed, 1);
        assert_eq!(vault.search_files("timeout", false)?.len(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn copy_file_to_second_local_backend_adds_locations() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        let backup_dir = root.join("backup");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("copy.txt");
        fs::write(&input, b"copy this chunk\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/copy.txt".to_owned()),
        })?;
        vault.add_local_backend("backup", backup_dir.clone())?;
        let copied = vault.copy_file_to_backend(&manifest.file_id, "backup")?;

        assert_eq!(copied.chunks.len(), 1);
        assert_eq!(copied.chunks[0].locations.len(), 2);
        assert!(copied.chunks[0]
            .locations
            .iter()
            .any(|location| location.backend_id == "backup"));

        let locations = vault.file_locations(&manifest.file_id)?;
        assert_eq!(locations.len(), 2);
        let backup_location = locations
            .iter()
            .find(|location| location.backend_id == "backup")
            .expect("backup location");
        assert!(backup_dir.join(&backup_location.object_key).exists());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn storage_move_verify_remove_and_policy() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        let backup_dir = root.join("backup");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("move.txt");
        fs::write(&input, b"move this chunk\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/move.txt".to_owned()),
        })?;
        vault.add_local_backend("backup", backup_dir.clone())?;

        let only_location = vault.remove_file_location(&manifest.file_id, "local", false);
        assert!(only_location.is_err());

        vault.copy_file_to_backend(&manifest.file_id, "backup")?;
        let verify = vault.verify_file_locations(&manifest.file_id)?;
        assert_eq!(verify.len(), 2);
        assert!(verify.iter().all(|record| record.ok));

        vault.remove_file_location(&manifest.file_id, "local", false)?;
        let locations = vault.file_locations(&manifest.file_id)?;
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].backend_id, "backup");

        let moved = vault.move_file_to_backend(&manifest.file_id, "local")?;
        assert_eq!(moved.chunks[0].locations.len(), 1);
        assert_eq!(moved.chunks[0].locations[0].backend_id, "local");

        let policy = vault.set_replica_policy(
            &manifest.file_id,
            manifest::ReplicaPolicyMode::SingleCopy,
            1,
            vec!["local".to_owned()],
            vec![],
            None,
        )?;
        assert!(policy.replica_policy.is_some());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn s3_backend_config_uses_prefixed_object_keys() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let vault = Vault::init(&root)?;
        vault.add_s3_backend(
            "minio",
            "http://localhost:9000".to_owned(),
            "deltabox".to_owned(),
            "us-east-1".to_owned(),
            "access".to_owned(),
            "secret".to_owned(),
            Some("chunks".to_owned()),
            true,
            true,
        )?;

        let backends = vault.list_backends()?;
        assert!(backends
            .iter()
            .any(|backend| backend.backend_id == "minio" && backend.backend_type == "s3"));

        let object_key = vault.object_key_for_backend_chunk("minio", "sha256:abcdef1234567890")?;
        assert_eq!(object_key, "chunks/ab/cdef1234567890");

        let minio = backends
            .iter()
            .find(|backend| backend.backend_id == "minio")
            .expect("minio backend");
        assert!(!minio.config_json.contains("access"));
        assert!(!minio.config_json.contains("secret"));
        assert!(vault.storage_backend_by_id("minio").is_ok());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn docx_text_is_indexed_from_document_xml() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("draft.docx");
        fs::write(
            &input,
            minimal_docx_with_paragraphs(&[
                "DeltaBox DOCX library planning notes",
                "Table-like project budget text",
            ])?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/draft.docx".to_owned()),
        })?;
        assert_eq!(
            manifest.mime,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
        assert_eq!(vault.search_files("library planning", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "docx_text");
        assert!(segments[0].text.contains("DOCX library planning"));
        assert!(segments[0].text.contains("project budget"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn docx_header_and_footer_text_are_indexed_separately() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("letter.docx");
        fs::write(
            &input,
            minimal_docx_with_parts(
                &["regular body text"],
                &["confidential library header"],
                &["archive footer marker"],
            )?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/letter.docx".to_owned()),
        })?;
        assert_eq!(vault.search_files("library header", false)?.len(), 1);
        assert_eq!(vault.search_files("footer marker", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 3);
        assert!(segments.iter().any(|segment| {
            segment.source == "docx_text" && segment.text.contains("regular body")
        }));
        let header = segments
            .iter()
            .find(|segment| segment.task_key == "docx:header_footer:1")
            .expect("header segment");
        assert_eq!(header.source, "docx_header_footer");
        assert!(header.text.contains("confidential library header"));
        let footer = segments
            .iter()
            .find(|segment| segment.task_key == "docx:header_footer:2")
            .expect("footer segment");
        assert_eq!(footer.source, "docx_header_footer");
        assert!(footer.text.contains("archive footer marker"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn xlsx_text_is_indexed_from_shared_strings_and_worksheets() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("budget.xlsx");
        fs::write(
            &input,
            minimal_xlsx_with_rows(&[
                &["DeltaBox XLSX library budget", "Q1 planning"],
                &["inline worksheet note", "128"],
            ])?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/budget.xlsx".to_owned()),
        })?;
        assert_eq!(
            manifest.mime,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(vault.search_files("library budget", false)?.len(), 1);
        assert_eq!(vault.search_files("worksheet note", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source, "xlsx_text");
        assert!(segments[0].text.contains("XLSX library budget"));
        assert!(segments[0].text.contains("inline worksheet note"));
        assert!(segments[0].text.contains("128"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn corrupt_docx_import_succeeds_and_records_index_error() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("corrupt.docx");
        fs::write(&input, b"not a zip archive")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/corrupt.docx".to_owned()),
        })?;
        assert_eq!(vault.list_files(false)?.len(), 1);
        assert_eq!(vault.text_segments_for_file(&manifest.file_id)?.len(), 0);

        let jobs = vault.list_index_jobs()?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].file_id, manifest.file_id);
        assert_eq!(jobs[0].status, "failed_permanent");
        assert!(jobs[0]
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("planning_failed"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn docx_without_document_xml_imports_as_no_text() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("empty.docx");
        fs::write(&input, minimal_zip_file("docProps/core.xml", "<core />")?)?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/empty.docx".to_owned()),
        })?;
        assert_eq!(vault.list_files(false)?.len(), 1);
        assert_eq!(vault.text_segments_for_file(&manifest.file_id)?.len(), 0);

        let jobs = vault.list_index_jobs()?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].file_id, manifest.file_id);
        assert_eq!(jobs[0].status, "skipped");
        assert_eq!(jobs[0].last_error.as_deref(), Some("no_text_tasks"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn corrupt_xlsx_import_succeeds_and_records_index_error() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("broken.xlsx");
        fs::write(&input, b"not a zip archive")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/broken.xlsx".to_owned()),
        })?;
        assert_eq!(vault.list_files(false)?.len(), 1);
        assert_eq!(vault.text_segments_for_file(&manifest.file_id)?.len(), 0);

        let jobs = vault.list_index_jobs()?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].file_id, manifest.file_id);
        assert_eq!(jobs[0].status, "failed_permanent");
        assert!(jobs[0]
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("planning_failed"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn pptx_text_is_indexed_by_slide() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("deck.pptx");
        fs::write(
            &input,
            minimal_pptx_with_slides(&[
                &["DeltaBox PPTX library overview", "first slide"],
                &["Roadmap budget planning", "second slide"],
            ])?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/deck.pptx".to_owned()),
        })?;
        assert_eq!(
            manifest.mime,
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        );
        assert_eq!(vault.search_files("library overview", false)?.len(), 1);
        assert_eq!(vault.search_files("budget planning", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].source, "pptx_text");
        assert_eq!(segments[0].task_key, "pptx:slide:1");
        assert_eq!(segments[0].page, Some(1));
        assert!(segments[0].text.contains("PPTX library overview"));
        assert_eq!(segments[1].task_key, "pptx:slide:2");
        assert_eq!(segments[1].page, Some(2));
        assert!(segments[1].text.contains("Roadmap budget planning"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn pptx_index_tasks_resume_slide_by_slide() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("deck-two-slides.pptx");
        fs::write(
            &input,
            minimal_pptx_with_slides(&[&["first slide archive"], &["second slide roadmap"]])?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/deck-two-slides.pptx".to_owned()),
        })?;

        let job = vault.enqueue_index_file(&manifest.file_id)?;
        assert_eq!(job.total_tasks, 2);
        assert_eq!(job.completed_tasks, 0);

        let summary = vault.run_index_worker(1)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.completed_tasks, 1);
        assert_eq!(job.status, "pending");
        assert!(vault.search_files("roadmap", false)?.is_empty());

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.completed_tasks, 2);
        assert_eq!(job.status, "completed");
        assert_eq!(vault.search_files("roadmap", false)?.len(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn pptx_speaker_notes_are_indexed_separately() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("deck-notes.pptx");
        fs::write(
            &input,
            minimal_pptx_with_slides_and_notes(
                &[&["visible slide title"]],
                &[&["private roadmap planning notes"]],
            )?,
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/deck-notes.pptx".to_owned()),
        })?;
        assert_eq!(vault.search_files("roadmap planning", false)?.len(), 1);

        let segments = vault.text_segments_for_file(&manifest.file_id)?;
        assert_eq!(segments.len(), 2);
        assert!(segments.iter().any(|segment| segment.source == "pptx_text"
            && segment.task_key == "pptx:slide:1"
            && segment.page == Some(1)));
        let notes = segments
            .iter()
            .find(|segment| segment.source == "pptx_speaker_notes")
            .expect("speaker notes segment");
        assert_eq!(notes.task_key, "pptx:notes:1");
        assert_eq!(notes.page, Some(1));
        assert!(notes.text.contains("private roadmap planning notes"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn corrupt_pptx_import_succeeds_and_records_index_error() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("broken.pptx");
        fs::write(&input, b"not a zip archive")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/broken.pptx".to_owned()),
        })?;
        assert_eq!(vault.list_files(false)?.len(), 1);
        assert_eq!(vault.text_segments_for_file(&manifest.file_id)?.len(), 0);

        let jobs = vault.list_index_jobs()?;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].file_id, manifest.file_id);
        assert_eq!(jobs[0].status, "failed_permanent");
        assert!(jobs[0]
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("planning_failed"));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn pdf_text_layer_is_indexed_with_page_locator() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("library-plan.pdf");
        fs::write(
            &input,
            minimal_pdf_with_pages(&["DeltaBox PDF library planning notes"]),
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/library-plan.pdf".to_owned()),
        })?;
        assert_eq!(manifest.mime, "application/pdf");

        let search = vault.search_files("library", false)?;
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].file_id, manifest.file_id);

        let detailed = vault.search_files_detailed("library", false)?;
        assert_eq!(detailed.len(), 1);
        assert_eq!(detailed[0].file.file_id, manifest.file_id);
        let text_match = detailed[0]
            .matches
            .iter()
            .find(|search_match| search_match.match_kind == "text")
            .expect("text match");
        assert_eq!(text_match.source.as_deref(), Some("pdf_text"));
        assert_eq!(text_match.page, Some(1));
        assert!(text_match
            .text
            .as_deref()
            .unwrap_or_default()
            .contains("library planning"));

        let conn = vault.open_db()?;
        let (source, page): (String, Option<u64>) = conn.query_row(
            "SELECT source, page FROM text_segments WHERE file_id = ?1",
            [&manifest.file_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(source, "pdf_text");
        assert_eq!(page, Some(1));

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn pdf_index_tasks_resume_page_by_page() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("two-pages.pdf");
        fs::write(
            &input,
            minimal_pdf_with_pages(&["first page library planning", "second page roadmap budget"]),
        )?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/two-pages.pdf".to_owned()),
        })?;

        let job = vault.enqueue_index_file(&manifest.file_id)?;
        assert_eq!(job.total_tasks, 2);
        assert_eq!(job.completed_tasks, 0);

        let summary = vault.run_index_worker(1)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.total_tasks, 2);
        assert_eq!(job.completed_tasks, 1);
        assert_eq!(job.status, "pending");

        let conn = vault.open_db()?;
        let indexed_pages: u64 = conn.query_row(
            "SELECT COUNT(DISTINCT page) FROM text_segments WHERE file_id = ?1",
            [&manifest.file_id],
            |row| row.get(0),
        )?;
        assert_eq!(indexed_pages, 1);

        let summary = vault.run_index_worker(10)?;
        assert_eq!(summary.completed, 1);
        let jobs = vault.list_index_jobs()?;
        let job = jobs
            .iter()
            .find(|candidate| candidate.job_id == job.job_id)
            .expect("index job");
        assert_eq!(job.completed_tasks, 2);
        assert_eq!(job.status, "completed");
        assert_eq!(vault.search_files("roadmap", false)?.len(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    fn minimal_pdf_with_pages(pages: &[&str]) -> Vec<u8> {
        let page_count = pages.len();
        let page_object_start = 4_usize;
        let content_object_start = page_object_start + page_count;
        let page_refs = (0..page_count)
            .map(|index| format!("{} 0 R", page_object_start + index))
            .collect::<Vec<_>>()
            .join(" ");
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            format!("<< /Type /Pages /Kids [{page_refs}] /Count {page_count} >>"),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        ];
        for index in 0..page_count {
            let content_id = content_object_start + index;
            objects.push(format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
            ));
        }
        for text in pages {
            let escaped = text
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            let stream = format!("BT\n/F1 18 Tf\n72 720 Td\n({escaped}) Tj\nET\n");
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}endstream",
                stream.len(),
                stream
            ));
        }

        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }

        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref_offset
            )
            .as_bytes(),
        );
        pdf
    }

    fn minimal_docx_with_paragraphs(paragraphs: &[&str]) -> Result<Vec<u8>> {
        minimal_docx_with_parts(paragraphs, &[], &[])
    }

    fn minimal_docx_with_parts(
        paragraphs: &[&str],
        headers: &[&str],
        footers: &[&str],
    ) -> Result<Vec<u8>> {
        let document = minimal_docx_xml("document", "body", paragraphs);

        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("word/document.xml", options)?;
        writer.write_all(document.as_bytes())?;
        if !headers.is_empty() {
            let header = minimal_docx_xml("hdr", "hdr", headers);
            writer.start_file("word/header1.xml", options)?;
            writer.write_all(header.as_bytes())?;
        }
        if !footers.is_empty() {
            let footer = minimal_docx_xml("ftr", "ftr", footers);
            writer.start_file("word/footer1.xml", options)?;
            writer.write_all(footer.as_bytes())?;
        }
        Ok(writer.finish()?.into_inner())
    }

    fn minimal_docx_xml(root: &str, container: &str, paragraphs: &[&str]) -> String {
        let mut xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:{root} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:{container}>
"#
        );
        for paragraph in paragraphs {
            xml.push_str("    <w:p><w:r><w:t>");
            xml.push_str(&escape_xml_text(paragraph));
            xml.push_str("</w:t></w:r></w:p>\n");
        }
        xml.push_str(&format!("  </w:{container}>\n</w:{root}>\n"));
        xml
    }

    fn minimal_xlsx_with_rows(rows: &[&[&str]]) -> Result<Vec<u8>> {
        let shared_strings = rows
            .iter()
            .flat_map(|row| row.iter())
            .enumerate()
            .map(|(_index, value)| format!(r#"<si><t>{}</t></si>"#, escape_xml_text(value)))
            .collect::<String>();
        let mut sheet = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
"#,
        );
        let mut shared_index = 0_usize;
        for (row_index, row) in rows.iter().enumerate() {
            sheet.push_str(&format!(r#"    <row r="{}">"#, row_index + 1));
            for (column_index, value) in row.iter().enumerate() {
                let column = (b'A' + column_index as u8) as char;
                if row_index == 1 && column_index == 0 {
                    sheet.push_str(&format!(
                        r#"<c r="{column}{}" t="inlineStr"><is><t>{}</t></is></c>"#,
                        row_index + 1,
                        escape_xml_text(value)
                    ));
                } else {
                    sheet.push_str(&format!(
                        r#"<c r="{column}{}" t="s"><v>{shared_index}</v></c>"#,
                        row_index + 1
                    ));
                }
                shared_index += 1;
            }
            sheet.push_str("</row>\n");
        }
        sheet.push_str("  </sheetData>\n</worksheet>\n");

        let shared = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{0}" uniqueCount="{0}">
{shared_strings}
</sst>
"#,
            shared_index
        );

        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file("xl/sharedStrings.xml", options)?;
        writer.write_all(shared.as_bytes())?;
        writer.start_file("xl/worksheets/sheet1.xml", options)?;
        writer.write_all(sheet.as_bytes())?;
        Ok(writer.finish()?.into_inner())
    }

    fn minimal_pptx_with_slides(slides: &[&[&str]]) -> Result<Vec<u8>> {
        minimal_pptx_with_slides_and_notes(slides, &[])
    }

    fn minimal_pptx_with_slides_and_notes(
        slides: &[&[&str]],
        notes: &[&[&str]],
    ) -> Result<Vec<u8>> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        for (index, slide_texts) in slides.iter().enumerate() {
            let slide = minimal_pptx_xml(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
"#,
                "  </p:spTree></p:cSld>\n</p:sld>\n",
                slide_texts,
            );
            writer.start_file(format!("ppt/slides/slide{}.xml", index + 1), options)?;
            writer.write_all(slide.as_bytes())?;
        }
        for (index, note_texts) in notes.iter().enumerate() {
            let note = minimal_pptx_xml(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:notes xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
         xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
"#,
                "  </p:spTree></p:cSld>\n</p:notes>\n",
                note_texts,
            );
            writer.start_file(
                format!("ppt/notesSlides/notesSlide{}.xml", index + 1),
                options,
            )?;
            writer.write_all(note.as_bytes())?;
        }

        Ok(writer.finish()?.into_inner())
    }

    fn minimal_pptx_xml(prefix: &str, suffix: &str, text_runs: &[&str]) -> String {
        let mut xml = prefix.to_owned();
        for text in text_runs {
            xml.push_str("    <p:sp><p:txBody><a:p><a:r><a:t>");
            xml.push_str(&escape_xml_text(text));
            xml.push_str("</a:t></a:r></a:p></p:txBody></p:sp>\n");
        }
        xml.push_str(suffix);
        xml
    }

    fn minimal_zip_file(name: &str, content: &str) -> Result<Vec<u8>> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        writer.start_file(name, options)?;
        writer.write_all(content.as_bytes())?;
        Ok(writer.finish()?.into_inner())
    }

    fn escape_xml_text(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}
