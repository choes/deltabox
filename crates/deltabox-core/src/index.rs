use std::path::Path;

use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, OptionalExtension};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::extractors::{
    extractor_for_manifest, is_text_extractable, ExtractedTextSegment, ExtractionTask,
};
use crate::manifest::FileManifest;
use crate::util::now_rfc3339;
use crate::{IndexJobRecord, TextSegmentRecord, Vault};

const DEFAULT_STALE_TIMEOUT_SECONDS: i64 = 600;

#[derive(Debug, Clone)]
pub struct IndexRunSummary {
    pub completed: u64,
    pub failed: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone)]
struct IndexTaskRecord {
    job_id: String,
    file_id: String,
    task_type: String,
    task_key: String,
}

impl Vault {
    pub fn index_file(&self, file_id: &str) -> Result<IndexJobRecord> {
        let job = self.enqueue_index_file(file_id)?;
        self.run_index_job(&job.job_id)?;
        self.get_index_job(&job.job_id)
    }

    pub fn enqueue_index_file(&self, file_id: &str) -> Result<IndexJobRecord> {
        self.ensure_file_exists(file_id)?;
        let manifest = self.get_manifest(file_id)?;
        if !is_text_extractable(&manifest) {
            return self.create_skipped_index_job(&manifest, "unsupported_mime");
        }

        let extractor = extractor_for_manifest(&manifest)
            .ok_or_else(|| anyhow!("unsupported mime type: {}", manifest.mime))?;
        let bytes = self.read_file_bytes(&manifest)?;
        let tasks = match extractor.plan_tasks(&manifest, &bytes) {
            Ok(tasks) => tasks,
            Err(error) => {
                return self.create_failed_index_job(
                    &manifest,
                    "planning_failed",
                    &error.to_string(),
                );
            }
        };
        if tasks.is_empty() {
            return self.create_skipped_index_job(&manifest, "no_text_tasks");
        }

        self.delete_text_segments_for_file(&self.open_db()?, &manifest.file_id)?;
        let job =
            self.create_index_job(&manifest, "document_text", tasks.len() as u64, "pending")?;
        for task in tasks {
            self.upsert_index_task(
                &job.job_id,
                &manifest.file_id,
                "document_text",
                &task.task_key,
                "pending",
                None,
            )?;
        }
        Ok(job)
    }

    pub fn rebuild_text_index(&self) -> Result<Vec<IndexJobRecord>> {
        self.clear_text_index()?;
        let files = self.list_files(false)?;
        let mut jobs = Vec::new();
        for file in files {
            let manifest = self.get_manifest(&file.file_id)?;
            if is_text_extractable(&manifest) {
                let job = self.enqueue_index_file(&file.file_id)?;
                self.run_index_job(&job.job_id)?;
                jobs.push(self.get_index_job(&job.job_id)?);
            }
        }
        Ok(jobs)
    }

    pub fn list_index_jobs(&self) -> Result<Vec<IndexJobRecord>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT job_id, file_id, job_type, status, total_tasks, completed_tasks, failed_tasks, created_at, updated_at, last_error
             FROM index_jobs
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], row_to_index_job_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn text_segments_for_file(&self, file_id: &str) -> Result<Vec<TextSegmentRecord>> {
        self.ensure_file_exists(file_id)?;
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT segment_id, file_id, source, task_key, segment_index, text,
                    page, line_start, line_end, char_start, char_end, start_ms, end_ms,
                    confidence, created_at, updated_at
             FROM text_segments
             WHERE file_id = ?1
             ORDER BY segment_index, page, line_start",
        )?;
        let rows = stmt.query_map(params![file_id], row_to_text_segment_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    pub fn run_index_worker(&self, limit: u64) -> Result<IndexRunSummary> {
        self.run_index_worker_with_stale_timeout(limit, DEFAULT_STALE_TIMEOUT_SECONDS)
    }

    pub fn run_index_worker_with_stale_timeout(
        &self,
        limit: u64,
        stale_timeout_seconds: i64,
    ) -> Result<IndexRunSummary> {
        self.requeue_stale_running_tasks(stale_timeout_seconds)?;
        let tasks = self.pending_index_tasks(limit)?;
        let mut summary = IndexRunSummary {
            completed: 0,
            failed: 0,
            skipped: 0,
        };

        for task in tasks {
            match self.run_index_task(&task) {
                Ok(()) => summary.completed += 1,
                Err(_) => summary.failed += 1,
            }
        }

        Ok(summary)
    }

    pub fn pause_index_job(&self, job_id: &str) -> Result<IndexJobRecord> {
        self.get_index_job(job_id)?;
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'paused', updated_at = ?1
             WHERE job_id = ?2 AND status IN ('pending', 'running', 'failed_retryable')",
            params![now, job_id],
        )?;
        conn.execute(
            "UPDATE index_jobs
             SET status = 'paused', updated_at = ?1
             WHERE job_id = ?2 AND status IN ('pending', 'running', 'failed_retryable')",
            params![now, job_id],
        )?;
        self.get_index_job(job_id)
    }

    pub fn resume_index_job(&self, job_id: &str) -> Result<IndexJobRecord> {
        self.get_index_job(job_id)?;
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'pending', updated_at = ?1
             WHERE job_id = ?2 AND status = 'paused'",
            params![now, job_id],
        )?;
        conn.execute(
            "UPDATE index_jobs
             SET status = 'pending', updated_at = ?1
             WHERE job_id = ?2 AND status = 'paused'",
            params![now, job_id],
        )?;
        self.get_index_job(job_id)
    }

    pub fn retry_index_job(&self, job_id: &str) -> Result<IndexJobRecord> {
        self.get_index_job(job_id)?;
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'pending', updated_at = ?1, last_error = NULL
             WHERE job_id = ?2 AND status IN ('failed_retryable', 'failed_permanent', 'cancelled')",
            params![now, job_id],
        )?;
        conn.execute(
            "UPDATE index_jobs
             SET status = 'pending', updated_at = ?1, last_error = NULL
             WHERE job_id = ?2 AND status IN ('failed_retryable', 'failed_permanent', 'cancelled')",
            params![now, job_id],
        )?;
        self.get_index_job(job_id)
    }

    pub fn cancel_index_job(&self, job_id: &str) -> Result<IndexJobRecord> {
        self.get_index_job(job_id)?;
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'cancelled', updated_at = ?1
             WHERE job_id = ?2 AND status IN ('pending', 'running', 'failed_retryable')",
            params![now, job_id],
        )?;
        self.finish_index_job(job_id, "cancelled", 0, 0, Some("cancelled"))
    }

    pub(crate) fn index_text_file_from_path(
        &self,
        manifest: &FileManifest,
        _path: &Path,
    ) -> Result<()> {
        if !is_text_extractable(manifest) {
            return Ok(());
        }
        let Ok(job) = self.enqueue_index_file(&manifest.file_id) else {
            return Ok(());
        };
        let _ = self.run_index_job(&job.job_id);
        Ok(())
    }

    fn run_index_job(&self, job_id: &str) -> Result<()> {
        loop {
            let Some(task) = self.next_task_for_job(job_id)? else {
                break;
            };
            self.run_index_task(&task)?;
        }
        Ok(())
    }

    fn run_index_task(&self, task: &IndexTaskRecord) -> Result<()> {
        self.mark_index_task_running(task)?;
        let result = self.execute_index_task(task);
        match result {
            Ok(()) => {
                self.mark_index_task_completed(task)?;
                self.update_job_progress(&task.job_id)?;
                Ok(())
            }
            Err(error) => {
                self.mark_index_task_failed(task, &error.to_string())?;
                self.update_job_progress(&task.job_id)?;
                Err(error)
            }
        }
    }

    fn execute_index_task(&self, task: &IndexTaskRecord) -> Result<()> {
        if task.task_type != "document_text" {
            return Err(anyhow!("unsupported index task type: {}", task.task_type));
        }
        let manifest = self.get_manifest(&task.file_id)?;
        let extractor = extractor_for_manifest(&manifest)
            .ok_or_else(|| anyhow!("unsupported mime type: {}", manifest.mime))?;
        let bytes = self.read_file_bytes(&manifest)?;
        let extraction_task = ExtractionTask {
            task_key: task.task_key.clone(),
        };
        let segments = extractor.extract_task(&manifest, &bytes, &extraction_task)?;
        self.replace_text_segments_for_task(&manifest, &task.task_key, segments)?;
        self.record_event(
            "index.updated",
            Some(&manifest.file_id),
            Some(&manifest.logical_path),
        )?;
        Ok(())
    }

    fn replace_text_segments_for_task(
        &self,
        manifest: &FileManifest,
        task_key: &str,
        segments: Vec<ExtractedTextSegment>,
    ) -> Result<()> {
        let conn = self.open_db()?;
        self.delete_text_segments_for_task(&conn, &manifest.file_id, task_key)?;

        for segment in segments {
            let segment_id = Uuid::new_v4().to_string();
            let now = now_rfc3339();
            conn.execute(
                "INSERT INTO text_segments (
                    segment_id, file_id, source, task_key, segment_index, text,
                    page, line_start, line_end, char_start, char_end, start_ms, end_ms,
                    confidence, created_at, updated_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    &segment_id,
                    &manifest.file_id,
                    &segment.source,
                    &segment.task_key,
                    segment.segment_index,
                    &segment.text,
                    segment.page,
                    segment.line_start,
                    segment.line_end,
                    segment.char_start,
                    segment.char_end,
                    segment.start_ms,
                    segment.end_ms,
                    segment.confidence,
                    now,
                    now,
                ],
            )?;
            conn.execute(
                "INSERT INTO text_segments_fts (segment_id, file_id, text) VALUES (?1, ?2, ?3)",
                params![&segment_id, &manifest.file_id, &segment.text],
            )?;
        }
        Ok(())
    }

    fn delete_text_segments_for_task(
        &self,
        conn: &Connection,
        file_id: &str,
        task_key: &str,
    ) -> Result<()> {
        let mut stmt = conn
            .prepare("SELECT segment_id FROM text_segments WHERE file_id = ?1 AND task_key = ?2")?;
        let segment_ids = stmt
            .query_map(params![file_id, task_key], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for segment_id in segment_ids {
            conn.execute(
                "DELETE FROM text_segments_fts WHERE segment_id = ?1",
                params![segment_id],
            )?;
        }
        conn.execute(
            "DELETE FROM text_segments WHERE file_id = ?1 AND task_key = ?2",
            params![file_id, task_key],
        )?;
        Ok(())
    }

    fn delete_text_segments_for_file(&self, conn: &Connection, file_id: &str) -> Result<()> {
        let mut stmt = conn.prepare("SELECT segment_id FROM text_segments WHERE file_id = ?1")?;
        let segment_ids = stmt
            .query_map(params![file_id], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for segment_id in segment_ids {
            conn.execute(
                "DELETE FROM text_segments_fts WHERE segment_id = ?1",
                params![segment_id],
            )?;
        }
        conn.execute(
            "DELETE FROM text_segments WHERE file_id = ?1",
            params![file_id],
        )?;
        Ok(())
    }

    fn clear_text_index(&self) -> Result<()> {
        let conn = self.open_db()?;
        conn.execute("DELETE FROM text_segments_fts", [])?;
        conn.execute("DELETE FROM text_segments", [])?;
        Ok(())
    }

    fn create_index_job(
        &self,
        manifest: &FileManifest,
        job_type: &str,
        total_tasks: u64,
        status: &str,
    ) -> Result<IndexJobRecord> {
        let now = now_rfc3339();
        let job = IndexJobRecord {
            job_id: Uuid::new_v4().to_string(),
            file_id: manifest.file_id.clone(),
            job_type: job_type.to_owned(),
            status: status.to_owned(),
            total_tasks,
            completed_tasks: 0,
            failed_tasks: 0,
            created_at: now.clone(),
            updated_at: now,
            last_error: None,
        };
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO index_jobs (job_id, file_id, job_type, status, total_tasks, completed_tasks, failed_tasks, created_at, updated_at, last_error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                job.job_id,
                job.file_id,
                job.job_type,
                job.status,
                job.total_tasks,
                job.completed_tasks,
                job.failed_tasks,
                job.created_at,
                job.updated_at,
                job.last_error,
            ],
        )?;
        Ok(job)
    }

    fn create_skipped_index_job(
        &self,
        manifest: &FileManifest,
        reason: &str,
    ) -> Result<IndexJobRecord> {
        let job = self.create_index_job(manifest, "document_text", 0, "skipped")?;
        self.finish_index_job(&job.job_id, "skipped", 0, 0, Some(reason))
    }

    fn create_failed_index_job(
        &self,
        manifest: &FileManifest,
        reason: &str,
        error: &str,
    ) -> Result<IndexJobRecord> {
        let message = format!("{reason}: {error}");
        let job = self.create_index_job(manifest, "document_text", 0, "failed_permanent")?;
        self.finish_index_job(&job.job_id, "failed_permanent", 0, 0, Some(&message))
    }

    fn finish_index_job(
        &self,
        job_id: &str,
        status: &str,
        completed_tasks: u64,
        failed_tasks: u64,
        last_error: Option<&str>,
    ) -> Result<IndexJobRecord> {
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_jobs
             SET status = ?1, completed_tasks = ?2, failed_tasks = ?3, updated_at = ?4, last_error = ?5
             WHERE job_id = ?6",
            params![status, completed_tasks, failed_tasks, now, last_error, job_id],
        )?;
        self.get_index_job(job_id)
    }

    fn get_index_job(&self, job_id: &str) -> Result<IndexJobRecord> {
        let conn = self.open_db()?;
        conn.query_row(
            "SELECT job_id, file_id, job_type, status, total_tasks, completed_tasks, failed_tasks, created_at, updated_at, last_error
             FROM index_jobs WHERE job_id = ?1",
            params![job_id],
            row_to_index_job_record,
        )
        .map_err(Into::into)
    }

    fn upsert_index_task(
        &self,
        job_id: &str,
        file_id: &str,
        task_type: &str,
        task_key: &str,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<()> {
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "INSERT INTO index_tasks (
                task_id, job_id, file_id, task_type, task_key, status,
                attempt_count, input_hash, output_hash, started_at, completed_at, updated_at, last_error
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL, NULL, NULL, NULL, ?7, ?8)
             ON CONFLICT(job_id, task_key) DO UPDATE SET
                status = excluded.status,
                updated_at = excluded.updated_at,
                last_error = excluded.last_error",
            params![
                Uuid::new_v4().to_string(),
                job_id,
                file_id,
                task_type,
                task_key,
                status,
                now,
                last_error,
            ],
        )?;
        Ok(())
    }

    fn requeue_stale_running_tasks(&self, stale_timeout_seconds: i64) -> Result<()> {
        let conn = self.open_db()?;
        let now = now_rfc3339();
        let cutoff = stale_cutoff_rfc3339(stale_timeout_seconds)?;
        let job_ids = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT job_id
                 FROM index_tasks
                 WHERE status = 'running' AND updated_at <= ?1",
            )?;
            let rows = stmt.query_map(params![cutoff], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        conn.execute(
            "UPDATE index_tasks
             SET status = 'pending', updated_at = ?1
             WHERE status = 'running' AND updated_at <= ?2",
            params![now, cutoff],
        )?;
        for job_id in job_ids {
            self.update_job_progress(&job_id)?;
        }
        Ok(())
    }

    fn pending_index_tasks(&self, limit: u64) -> Result<Vec<IndexTaskRecord>> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT job_id, file_id, task_type, task_key
             FROM index_tasks
             WHERE status IN ('pending', 'failed_retryable')
             ORDER BY updated_at ASC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], row_to_index_task_record)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }

    fn next_task_for_job(&self, job_id: &str) -> Result<Option<IndexTaskRecord>> {
        let conn = self.open_db()?;
        conn.query_row(
            "SELECT job_id, file_id, task_type, task_key
             FROM index_tasks
             WHERE job_id = ?1 AND status IN ('pending', 'failed_retryable')
             ORDER BY updated_at ASC
             LIMIT 1",
            params![job_id],
            row_to_index_task_record,
        )
        .optional()
        .map_err(Into::into)
    }

    fn mark_index_task_running(&self, task: &IndexTaskRecord) -> Result<()> {
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'running', attempt_count = attempt_count + 1, started_at = ?1, updated_at = ?2, last_error = NULL
             WHERE job_id = ?3 AND task_key = ?4",
            params![now, now, task.job_id, task.task_key],
        )?;
        conn.execute(
            "UPDATE index_jobs SET status = 'running', updated_at = ?1 WHERE job_id = ?2",
            params![now, task.job_id],
        )?;
        Ok(())
    }

    fn mark_index_task_completed(&self, task: &IndexTaskRecord) -> Result<()> {
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'completed', completed_at = ?1, updated_at = ?2, last_error = NULL
             WHERE job_id = ?3 AND task_key = ?4",
            params![now, now, task.job_id, task.task_key],
        )?;
        Ok(())
    }

    fn mark_index_task_failed(&self, task: &IndexTaskRecord, error: &str) -> Result<()> {
        let now = now_rfc3339();
        let conn = self.open_db()?;
        conn.execute(
            "UPDATE index_tasks
             SET status = 'failed_retryable', updated_at = ?1, last_error = ?2
             WHERE job_id = ?3 AND task_key = ?4",
            params![now, error, task.job_id, task.task_key],
        )?;
        Ok(())
    }

    fn update_job_progress(&self, job_id: &str) -> Result<()> {
        let conn = self.open_db()?;
        let completed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM index_tasks WHERE job_id = ?1 AND status = 'completed'",
            params![job_id],
            |row| row.get(0),
        )?;
        let failed: u64 = conn.query_row(
            "SELECT COUNT(*) FROM index_tasks WHERE job_id = ?1 AND status IN ('failed_retryable', 'failed_permanent')",
            params![job_id],
            |row| row.get(0),
        )?;
        let total: u64 = conn.query_row(
            "SELECT total_tasks FROM index_jobs WHERE job_id = ?1",
            params![job_id],
            |row| row.get(0),
        )?;
        let status = if completed == total {
            "completed"
        } else if failed > 0 {
            "failed_retryable"
        } else {
            "pending"
        };
        let last_error: Option<String> = conn
            .query_row(
                "SELECT last_error FROM index_tasks WHERE job_id = ?1 AND last_error IS NOT NULL ORDER BY updated_at DESC LIMIT 1",
                params![job_id],
                |row| row.get(0),
            )
            .optional()?;
        conn.execute(
            "UPDATE index_jobs
             SET status = ?1, completed_tasks = ?2, failed_tasks = ?3, updated_at = ?4, last_error = ?5
             WHERE job_id = ?6",
            params![status, completed, failed, now_rfc3339(), last_error, job_id],
        )?;
        Ok(())
    }
}

fn row_to_index_job_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexJobRecord> {
    Ok(IndexJobRecord {
        job_id: row.get(0)?,
        file_id: row.get(1)?,
        job_type: row.get(2)?,
        status: row.get(3)?,
        total_tasks: row.get(4)?,
        completed_tasks: row.get(5)?,
        failed_tasks: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        last_error: row.get(9)?,
    })
}

fn row_to_index_task_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexTaskRecord> {
    Ok(IndexTaskRecord {
        job_id: row.get(0)?,
        file_id: row.get(1)?,
        task_type: row.get(2)?,
        task_key: row.get(3)?,
    })
}

fn row_to_text_segment_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<TextSegmentRecord> {
    Ok(TextSegmentRecord {
        segment_id: row.get(0)?,
        file_id: row.get(1)?,
        source: row.get(2)?,
        task_key: row.get(3)?,
        segment_index: row.get(4)?,
        text: row.get(5)?,
        page: row.get(6)?,
        line_start: row.get(7)?,
        line_end: row.get(8)?,
        char_start: row.get(9)?,
        char_end: row.get(10)?,
        start_ms: row.get(11)?,
        end_ms: row.get(12)?,
        confidence: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn stale_cutoff_rfc3339(stale_timeout_seconds: i64) -> Result<String> {
    let timeout = stale_timeout_seconds.max(0);
    let cutoff = OffsetDateTime::now_utc() - Duration::seconds(timeout);
    Ok(cutoff.format(&time::format_description::well_known::Rfc3339)?)
}
