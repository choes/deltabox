use std::fs;
use std::path::PathBuf;

use axum::body::{to_bytes, Body, Bytes};
use axum::http::{header, Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use deltabox_server::{build_router, AppState};

struct TestApp {
    root: PathBuf,
    router: Router,
}

impl TestApp {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("deltabox-server-test-{}", Uuid::new_v4()));
        let vault = deltabox_core::Vault::init(&root).expect("init vault");
        let router = build_router(AppState { vault }, root.join("no-static-dir"));
        Self { root, router }
    }

    async fn request(&self, req: Request<Body>) -> (StatusCode, Bytes) {
        let response = self.router.clone().oneshot(req).await.expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, body)
    }

    async fn json(&self, req: Request<Body>) -> (StatusCode, Value) {
        let (status, body) = self.request(req).await;
        let value = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).expect("json body")
        };
        (status, value)
    }

    async fn get_json(&self, path: &str) -> (StatusCode, Value) {
        self.json(
            Request::get(path)
                .body(Body::empty())
                .expect("build request"),
        )
        .await
    }

    async fn upload(&self, filename: &str, logical_path: &str, content: &[u8]) -> Value {
        let boundary = "deltaboxtest";
        let mut body = Vec::new();
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"path\"\r\n\r\n{logical_path}\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(content);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let (status, value) = self
            .json(
                Request::post("/api/files")
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .expect("build request"),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "upload failed: {value}");
        value
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn upload_list_info_and_download_roundtrip() {
    let app = TestApp::new();
    let content = b"hello deltabox h5 server";

    let manifest = app.upload("hello.txt", "/docs/hello.txt", content).await;
    let file_id = manifest["file_id"].as_str().expect("file_id");
    assert_eq!(manifest["name"], "hello.txt");

    let (status, files) = app.get_json("/api/files").await;
    assert_eq!(status, StatusCode::OK);
    assert!(files
        .as_array()
        .expect("files array")
        .iter()
        .any(|f| f["file_id"] == file_id));

    let (status, info) = app.get_json(&format!("/api/files/{file_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(info["logical_path"], "/docs/hello.txt");

    let (status, body, headers) = {
        let response = app
            .router
            .clone()
            .oneshot(
                Request::get(format!("/api/files/{file_id}/download"))
                    .body(Body::empty())
                    .expect("build request"),
            )
            .await
            .expect("response");
        let status = response.status();
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        (status, body, headers)
    };
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_ref(), content);
    assert!(headers
        .get(header::CONTENT_DISPOSITION)
        .expect("content-disposition")
        .to_str()
        .expect("header str")
        .contains("hello.txt"));
}

#[tokio::test]
async fn search_finds_uploaded_text() {
    let app = TestApp::new();
    app.upload("notes.txt", "/docs/notes.txt", b"unique keyword zephyr")
        .await;

    let (status, results) = app.get_json("/api/search?q=zephyr").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(results.as_array().expect("results").len(), 1);

    let (status, detailed) = app.get_json("/api/search?q=zephyr&details=true").await;
    assert_eq!(status, StatusCode::OK);
    let matches = &detailed[0]["matches"];
    assert!(matches
        .as_array()
        .expect("matches")
        .iter()
        .any(|m| m["match_kind"] == "text"));
}

#[tokio::test]
async fn tag_lifecycle() {
    let app = TestApp::new();
    let manifest = app.upload("doc.txt", "/docs/doc.txt", b"tag me").await;
    let file_id = manifest["file_id"].as_str().expect("file_id");

    let (status, tag) = app
        .json(
            Request::post("/api/tags")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "name": "工作规划", "tag_type": "project" }).to_string(),
                ))
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tag["name"], "工作规划");

    let encoded = "工作规划";
    let (status, _) = app
        .json(
            Request::post(format!("/api/files/{file_id}/tags"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "name": encoded }).to_string()))
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, tags) = app.get_json(&format!("/api/files/{file_id}/tags")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tags.as_array().expect("tags").len(), 1);

    let (status, _) = app
        .json(
            Request::delete(format!("/api/files/{file_id}/tags/工作规划"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (_, tags) = app.get_json(&format!("/api/files/{file_id}/tags")).await;
    assert!(tags.as_array().expect("tags").is_empty());

    let (status, renamed) = app
        .json(
            Request::put("/api/tags/工作规划")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "new_name": "年度规划" }).to_string()))
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(renamed["name"], "年度规划");

    let (status, _) = app
        .json(
            Request::delete("/api/tags/年度规划")
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn index_jobs_listing_and_actions() {
    let app = TestApp::new();
    app.upload("notes.txt", "/docs/notes.txt", b"index me")
        .await;

    let (status, jobs) = app.get_json("/api/index/jobs").await;
    assert_eq!(status, StatusCode::OK);
    let jobs = jobs.as_array().expect("jobs array");
    assert!(!jobs.is_empty());

    let job_id = jobs[0]["job_id"].as_str().expect("job_id");
    let (status, job) = app
        .json(
            Request::post(format!("/api/index/jobs/{job_id}/pause"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "pause failed: {job}");
    assert_eq!(job["job_id"], job_id);

    let (status, error) = app
        .json(
            Request::post(format!("/api/index/jobs/{job_id}/bogus"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(error["error"]
        .as_str()
        .expect("error message")
        .contains("unknown index job action"));

    let (status, summary) = app
        .json(
            Request::post("/api/index/run?limit=5")
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(summary["completed"].is_number());
}

#[tokio::test]
async fn missing_file_returns_404_json_error() {
    let app = TestApp::new();
    let (status, error) = app.get_json("/api/files/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(error["error"].as_str().expect("error message").len() > 0);
}

#[tokio::test]
async fn trash_file_marks_manifest_trashed() {
    let app = TestApp::new();
    let manifest = app.upload("doc.txt", "/docs/doc.txt", b"trash me").await;
    let file_id = manifest["file_id"].as_str().expect("file_id");

    let (status, trashed) = app
        .json(
            Request::delete(format!("/api/files/{file_id}"))
                .body(Body::empty())
                .expect("build request"),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(trashed["status"], "trashed");

    let (_, files) = app.get_json("/api/files").await;
    assert!(!files
        .as_array()
        .expect("files")
        .iter()
        .any(|f| f["file_id"] == file_id));
}
