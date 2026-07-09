pub(crate) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS files (
    file_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    logical_path TEXT NOT NULL,
    mime TEXT NOT NULL,
    size INTEGER NOT NULL,
    content_hash TEXT NOT NULL,
    version INTEGER NOT NULL,
    status TEXT NOT NULL,
    imported_at TEXT NOT NULL,
    trashed_at TEXT,
    manifest_path TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_files_status ON files(status);
CREATE INDEX IF NOT EXISTS idx_files_name ON files(name);
CREATE INDEX IF NOT EXISTS idx_files_path ON files(logical_path);

CREATE TABLE IF NOT EXISTS chunks (
    chunk_id TEXT PRIMARY KEY,
    hash TEXT NOT NULL,
    size INTEGER NOT NULL,
    backend_id TEXT NOT NULL,
    object_key TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS storage_backends (
    backend_id TEXT PRIMARY KEY,
    backend_type TEXT NOT NULL,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS backend_secrets (
    backend_id TEXT NOT NULL,
    secret_name TEXT NOT NULL,
    nonce_hex TEXT NOT NULL,
    encrypted_value_hex TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (backend_id, secret_name),
    FOREIGN KEY (backend_id) REFERENCES storage_backends(backend_id)
);

CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    file_id TEXT,
    subject_path TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    tag_id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    tag_type TEXT NOT NULL,
    source TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tags_name ON tags(name);

CREATE TABLE IF NOT EXISTS file_tags (
    file_id TEXT NOT NULL,
    tag_id TEXT NOT NULL,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (file_id, tag_id),
    FOREIGN KEY (file_id) REFERENCES files(file_id),
    FOREIGN KEY (tag_id) REFERENCES tags(tag_id)
);

CREATE INDEX IF NOT EXISTS idx_file_tags_file ON file_tags(file_id);
CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag_id);

CREATE TABLE IF NOT EXISTS text_segments (
    segment_id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    source TEXT NOT NULL,
    task_key TEXT NOT NULL,
    segment_index INTEGER NOT NULL,
    text TEXT NOT NULL,
    page INTEGER,
    line_start INTEGER,
    line_end INTEGER,
    char_start INTEGER,
    char_end INTEGER,
    start_ms INTEGER,
    end_ms INTEGER,
    confidence REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(file_id, source, task_key),
    FOREIGN KEY (file_id) REFERENCES files(file_id)
);

CREATE INDEX IF NOT EXISTS idx_text_segments_file ON text_segments(file_id);
CREATE INDEX IF NOT EXISTS idx_text_segments_source ON text_segments(source);

CREATE VIRTUAL TABLE IF NOT EXISTS text_segments_fts USING fts5(
    segment_id UNINDEXED,
    file_id UNINDEXED,
    text
);

CREATE TABLE IF NOT EXISTS index_jobs (
    job_id TEXT PRIMARY KEY,
    file_id TEXT NOT NULL,
    job_type TEXT NOT NULL,
    status TEXT NOT NULL,
    total_tasks INTEGER NOT NULL,
    completed_tasks INTEGER NOT NULL,
    failed_tasks INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_error TEXT,
    FOREIGN KEY (file_id) REFERENCES files(file_id)
);

CREATE INDEX IF NOT EXISTS idx_index_jobs_file ON index_jobs(file_id);
CREATE INDEX IF NOT EXISTS idx_index_jobs_status ON index_jobs(status);

CREATE TABLE IF NOT EXISTS index_tasks (
    task_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    file_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    task_key TEXT NOT NULL,
    status TEXT NOT NULL,
    attempt_count INTEGER NOT NULL,
    input_hash TEXT,
    output_hash TEXT,
    started_at TEXT,
    completed_at TEXT,
    updated_at TEXT NOT NULL,
    last_error TEXT,
    UNIQUE(job_id, task_key),
    FOREIGN KEY (job_id) REFERENCES index_jobs(job_id),
    FOREIGN KEY (file_id) REFERENCES files(file_id)
);

CREATE INDEX IF NOT EXISTS idx_index_tasks_job ON index_tasks(job_id);
CREATE INDEX IF NOT EXISTS idx_index_tasks_status ON index_tasks(status);
"#;
