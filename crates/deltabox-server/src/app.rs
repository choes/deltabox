use std::path::PathBuf;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{handlers, AppState};

/// Build the application router: JSON API under `/api`, static SPA hosting
/// for everything else (with `index.html` fallback for history-mode routing).
pub fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    let api = Router::new()
        .route("/files", get(handlers::list_files).post(handlers::upload_file))
        .route(
            "/files/{id}",
            get(handlers::file_info).delete(handlers::trash_file),
        )
        .route("/files/{id}/download", get(handlers::download_file))
        .route("/files/{id}/segments", get(handlers::file_segments))
        .route(
            "/files/{id}/tags",
            get(handlers::file_tags).post(handlers::attach_tag),
        )
        .route("/files/{id}/tags/{name}", delete(handlers::detach_tag))
        .route("/search", get(handlers::search))
        .route("/tags", get(handlers::list_tags).post(handlers::create_tag))
        .route(
            "/tags/{name}",
            put(handlers::rename_tag).delete(handlers::delete_tag),
        )
        .route("/index/jobs", get(handlers::list_index_jobs))
        .route("/index/jobs/{id}/{action}", post(handlers::index_job_action))
        .route("/index/run", post(handlers::run_index_worker))
        .with_state(state);

    // `fallback` (not `not_found_service`, which forces a 404 status) so the
    // SPA's index.html is served with 200 for client-side routes.
    let static_service =
        ServeDir::new(&static_dir).fallback(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        .nest("/api", api)
        .fallback_service(static_service)
}
