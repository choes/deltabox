#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="${DELTABOX_RUSTFS_CONTAINER:-deltabox-rustfs-it}"
BUCKET="${DELTABOX_S3_BUCKET:-deltabox-it}"
ACCESS_KEY="${DELTABOX_S3_ACCESS_KEY:-rustfsadmin}"
SECRET_KEY="${DELTABOX_S3_SECRET_KEY:-rustfsadmin}"
ENDPOINT="${DELTABOX_S3_ENDPOINT:-http://127.0.0.1:9000}"
AWS_CLI_IMAGE="${DELTABOX_AWS_CLI_IMAGE:-amazon/aws-cli:latest}"

if ! docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    docker start "${CONTAINER_NAME}" >/dev/null
  else
    docker run -d \
      --name "${CONTAINER_NAME}" \
      -p 9000:9000 \
      -p 9001:9001 \
      -e "RUSTFS_ACCESS_KEY=${ACCESS_KEY}" \
      -e "RUSTFS_SECRET_KEY=${SECRET_KEY}" \
      -e "RUSTFS_CONSOLE_ADDRESS=0.0.0.0:9001" \
      -v "deltabox-rustfs-it-data:/data" \
      rustfs/rustfs:latest >/dev/null
  fi
fi

aws_cli() {
  docker run --rm --network host \
    -e "AWS_ACCESS_KEY_ID=${ACCESS_KEY}" \
    -e "AWS_SECRET_ACCESS_KEY=${SECRET_KEY}" \
    -e "AWS_EC2_METADATA_DISABLED=true" \
    "${AWS_CLI_IMAGE}" --endpoint-url "${ENDPOINT}" "$@"
}

READY_ATTEMPTS=0
until aws_cli s3api list-buckets >/dev/null 2>&1; do
  READY_ATTEMPTS=$((READY_ATTEMPTS + 1))
  if [ "${READY_ATTEMPTS}" -ge 60 ]; then
    echo "RustFS did not become ready with the configured credentials." >&2
    echo "Check ${CONTAINER_NAME} or set DELTABOX_S3_ACCESS_KEY / DELTABOX_S3_SECRET_KEY." >&2
    exit 1
  fi
  sleep 1
done

aws_cli s3api head-bucket --bucket "${BUCKET}" >/dev/null 2>&1 || \
  aws_cli s3api create-bucket --bucket "${BUCKET}" >/dev/null

DELTABOX_RUN_RUSTFS_TESTS=1 \
DELTABOX_S3_ENDPOINT="${ENDPOINT}" \
DELTABOX_S3_BUCKET="${BUCKET}" \
DELTABOX_S3_ACCESS_KEY="${ACCESS_KEY}" \
DELTABOX_S3_SECRET_KEY="${SECRET_KEY}" \
cargo test --release -p deltabox-core --test rustfs_integration -- --ignored --nocapture
