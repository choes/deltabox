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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct TrashRecord {
    pub file_id: String,
    pub name: String,
    pub previous_path: String,
    pub size: u64,
    pub trashed_at: String,
}

#[derive(Debug, Clone)]
pub struct TagRecord {
    pub tag_id: String,
    pub name: String,
    pub tag_type: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct StorageBackendRecord {
    pub backend_id: String,
    pub backend_type: String,
    pub config_json: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ChunkLocationRecord {
    pub chunk_id: String,
    pub backend_id: String,
    pub object_key: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StorageVerifyRecord {
    pub chunk_id: String,
    pub backend_id: String,
    pub ok: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use std::fs;

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
    fn unsupported_document_type_creates_skipped_index_job() -> Result<()> {
        let root = std::env::temp_dir().join(format!("deltabox-core-test-{}", Uuid::new_v4()));
        let input_dir = root.join("input");
        fs::create_dir_all(&input_dir)?;

        let input = input_dir.join("draft.docx");
        fs::write(&input, b"not a real docx yet\n")?;

        let vault = Vault::init(&root)?;
        let manifest = vault.add_file(AddOptions {
            source: input,
            logical_path: Some("/docs/draft.docx".to_owned()),
        })?;
        assert_eq!(
            manifest.mime,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
        assert!(vault.list_index_jobs()?.is_empty());

        let job = vault.enqueue_index_file(&manifest.file_id)?;
        assert_eq!(job.status, "skipped");
        assert_eq!(job.last_error.as_deref(), Some("unsupported_mime"));

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
            minimal_pdf_with_text("DeltaBox PDF library planning notes"),
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

    fn minimal_pdf_with_text(text: &str) -> Vec<u8> {
        let escaped = text
            .replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)");
        let stream = format!("BT\n/F1 18 Tf\n72 720 Td\n({escaped}) Tj\nET\n");
        let objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                stream.as_bytes().len(),
                stream
            ),
        ];

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
}
