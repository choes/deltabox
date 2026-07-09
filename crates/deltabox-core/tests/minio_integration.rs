use std::env;
use std::fs;

use anyhow::Result;
use deltabox_core::{AddOptions, Vault};
use uuid::Uuid;

#[test]
#[ignore = "requires a running MinIO server and an existing bucket"]
fn minio_copy_verify_restore_and_move_roundtrip() -> Result<()> {
    if env::var("DELTABOX_RUN_MINIO_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping MinIO integration test; set DELTABOX_RUN_MINIO_TESTS=1");
        return Ok(());
    }

    let endpoint =
        env::var("DELTABOX_S3_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".to_owned());
    let bucket = env::var("DELTABOX_S3_BUCKET").unwrap_or_else(|_| "deltabox-it".to_owned());
    let region = env::var("DELTABOX_S3_REGION").unwrap_or_else(|_| "us-east-1".to_owned());
    let access_key = env::var("DELTABOX_S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_owned());
    let secret_key = env::var("DELTABOX_S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_owned());
    let prefix = env::var("DELTABOX_S3_PREFIX")
        .ok()
        .unwrap_or_else(|| format!("integration/{}", Uuid::new_v4()));

    let root = env::temp_dir().join(format!("deltabox-minio-test-{}", Uuid::new_v4()));
    let input_dir = root.join("input");
    let output_dir = root.join("output");
    fs::create_dir_all(&input_dir)?;
    fs::create_dir_all(&output_dir)?;

    let mut input_bytes = Vec::with_capacity(1_200_000);
    while input_bytes.len() < 1_200_000 {
        input_bytes.extend_from_slice(b"deltabox minio integration payload\n");
    }
    input_bytes.truncate(1_200_000);

    let input = input_dir.join("large-note.txt");
    fs::write(&input, &input_bytes)?;

    let vault = Vault::init(&root)?;
    vault.add_s3_backend(
        "minio",
        endpoint,
        bucket,
        region,
        access_key,
        secret_key,
        Some(prefix),
        true,
        true,
    )?;

    let manifest = vault.add_file(AddOptions {
        source: input,
        logical_path: Some("/integration/large-note.txt".to_owned()),
    })?;
    assert!(manifest.chunks.len() > 1);

    let copied = vault.copy_file_to_backend(&manifest.file_id, "minio")?;
    assert!(copied.chunks.iter().all(|chunk| {
        chunk
            .locations
            .iter()
            .any(|location| location.backend_id == "minio")
    }));

    let verify = vault.verify_file_locations(&manifest.file_id)?;
    assert!(verify.iter().all(|record| record.ok), "{verify:#?}");

    vault.remove_file_location(&manifest.file_id, "local", false)?;
    let s3_only_locations = vault.file_locations(&manifest.file_id)?;
    assert!(s3_only_locations
        .iter()
        .all(|location| location.backend_id == "minio"));

    let restored_from_minio =
        vault.restore_file(&manifest.file_id, output_dir.join("from-minio.txt"))?;
    assert_eq!(fs::read(restored_from_minio)?, input_bytes);

    let moved_back = vault.move_file_to_backend(&manifest.file_id, "local")?;
    assert!(moved_back
        .chunks
        .iter()
        .all(|chunk| chunk.locations.len() == 1 && chunk.locations[0].backend_id == "local"));

    let final_verify = vault.verify_file_locations(&manifest.file_id)?;
    assert!(
        final_verify.iter().all(|record| record.ok),
        "{final_verify:#?}"
    );

    fs::remove_dir_all(root)?;
    Ok(())
}
