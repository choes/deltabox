use anyhow::anyhow;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, HeaderValue},
    Json,
};
use serde::Deserialize;
use serde_json::Value;

use deltabox_core::AddOptions;

use crate::error::ApiResult;
use crate::AppState;

/// Run a synchronous core call on the blocking thread pool. The core is
/// fully sync (rusqlite + std::fs; S3 spins its own runtime internally),
/// so every vault operation must go through `spawn_blocking`.
async fn run<T, F>(f: F) -> ApiResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> anyhow::Result<T> + Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f).await??)
}

// ---- files ----

pub async fn list_files(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let files = run(move || state.vault.list_files(false)).await?;
    Ok(Json(serde_json::to_value(files)?))
}

pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> ApiResult<Json<Value>> {
    let mut file: Option<(String, Vec<u8>)> = None;
    let mut logical_path: Option<String> = None;
    while let Some(field) = multipart.next_field().await? {
        match field.name() {
            Some("file") => {
                let name = field
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "upload.bin".to_string());
                let data = field.bytes().await?;
                file = Some((name, data.to_vec()));
            }
            Some("path") => {
                logical_path = Some(field.text().await?);
            }
            _ => {}
        }
    }
    let (name, data) = file.ok_or_else(|| anyhow!("missing multipart field: file"))?;

    // add_file derives the file name from the source basename, so stage the
    // upload in a temp directory under its original name.
    let tmp_dir = std::env::temp_dir().join(format!("deltabox-upload-{}", uuid::Uuid::new_v4()));
    let tmp = tmp_dir.join(&name);
    std::fs::create_dir_all(&tmp_dir)?;
    std::fs::write(&tmp, &data)?;
    let logical_path = logical_path.or(Some(format!("/{name}")));
    let manifest = run(move || {
        let result = state
            .vault
            .add_file(AddOptions {
                source: tmp.clone(),
                logical_path,
            });
        let _ = std::fs::remove_dir_all(&tmp_dir);
        result
    })
    .await?;
    Ok(Json(serde_json::to_value(manifest)?))
}

pub async fn file_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let manifest = run(move || state.vault.get_manifest(&id)).await?;
    Ok(Json(serde_json::to_value(manifest)?))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<(HeaderMap, Vec<u8>)> {
    let (manifest, bytes) = run(move || {
        let manifest = state.vault.get_manifest(&id)?;
        let bytes = state.vault.read_file_bytes(&manifest)?;
        Ok((manifest, bytes))
    })
    .await?;

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&manifest.mime)
            .unwrap_or(HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = HeaderValue::from_str(&content_disposition(&manifest.name)) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    Ok((headers, bytes))
}

/// RFC 5987 `filename*` parameter so non-ASCII names survive.
fn content_disposition(filename: &str) -> String {
    let mut encoded = String::new();
    for &b in filename.as_bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(b as char)
            }
            _ => encoded.push_str(&format!("%{b:02X}")),
        }
    }
    format!("attachment; filename*=UTF-8''{encoded}")
}

pub async fn trash_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let manifest = run(move || state.vault.move_to_trash(&id)).await?;
    Ok(Json(serde_json::to_value(manifest)?))
}

// ---- search ----

#[derive(Deserialize)]
pub struct SearchParams {
    q: String,
    details: Option<bool>,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<Value>> {
    let query = params.q;
    let detailed = params.details.unwrap_or(false);
    let value = run(move || {
        if detailed {
            Ok(serde_json::to_value(
                state.vault.search_files_detailed(&query, false)?,
            )?)
        } else {
            Ok(serde_json::to_value(
                state.vault.search_files(&query, false)?,
            )?)
        }
    })
    .await?;
    Ok(Json(value))
}

// ---- segments ----

pub async fn file_segments(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let segments = run(move || state.vault.text_segments_for_file(&id)).await?;
    Ok(Json(serde_json::to_value(segments)?))
}

// ---- tags ----

pub async fn list_tags(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let tags = run(move || state.vault.list_tags()).await?;
    Ok(Json(serde_json::to_value(tags)?))
}

#[derive(Deserialize)]
pub struct CreateTagBody {
    name: String,
    tag_type: Option<String>,
}

pub async fn create_tag(
    State(state): State<AppState>,
    Json(body): Json<CreateTagBody>,
) -> ApiResult<Json<Value>> {
    let tag = run(move || {
        state
            .vault
            .create_tag(&body.name, body.tag_type.as_deref().unwrap_or("generic"))
    })
    .await?;
    Ok(Json(serde_json::to_value(tag)?))
}

#[derive(Deserialize)]
pub struct RenameTagBody {
    new_name: String,
}

pub async fn rename_tag(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<RenameTagBody>,
) -> ApiResult<Json<Value>> {
    let tag = run(move || state.vault.rename_tag(&name, &body.new_name)).await?;
    Ok(Json(serde_json::to_value(tag)?))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult<Json<Value>> {
    run(move || state.vault.delete_tag(&name)).await?;
    Ok(Json(Value::Null))
}

pub async fn file_tags(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Value>> {
    let tags = run(move || state.vault.tags_for_file(&id)).await?;
    Ok(Json(serde_json::to_value(tags)?))
}

#[derive(Deserialize)]
pub struct AttachTagBody {
    name: String,
}

pub async fn attach_tag(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AttachTagBody>,
) -> ApiResult<Json<Value>> {
    let tag = run(move || state.vault.attach_tag(&id, &body.name)).await?;
    Ok(Json(serde_json::to_value(tag)?))
}

pub async fn detach_tag(
    State(state): State<AppState>,
    Path((id, name)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    run(move || state.vault.detach_tag(&id, &name)).await?;
    Ok(Json(Value::Null))
}

// ---- index jobs ----

pub async fn list_index_jobs(State(state): State<AppState>) -> ApiResult<Json<Value>> {
    let jobs = run(move || state.vault.list_index_jobs()).await?;
    Ok(Json(serde_json::to_value(jobs)?))
}

pub async fn index_job_action(
    State(state): State<AppState>,
    Path((id, action)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    let job = run(move || match action.as_str() {
        "pause" => state.vault.pause_index_job(&id),
        "resume" => state.vault.resume_index_job(&id),
        "retry" => state.vault.retry_index_job(&id),
        "cancel" => state.vault.cancel_index_job(&id),
        _ => Err(anyhow!("unknown index job action: {action}")),
    })
    .await?;
    Ok(Json(serde_json::to_value(job)?))
}

#[derive(Deserialize)]
pub struct RunParams {
    limit: Option<u64>,
}

pub async fn run_index_worker(
    State(state): State<AppState>,
    Query(params): Query<RunParams>,
) -> ApiResult<Json<Value>> {
    let limit = params.limit.unwrap_or(10);
    let summary = run(move || state.vault.run_index_worker(limit)).await?;
    Ok(Json(serde_json::to_value(summary)?))
}
