#!/usr/bin/env bash
set -euo pipefail

CONTAINER_NAME="${DELTABOX_MINIO_CONTAINER:-deltabox-minio-it}"
BUCKET="${DELTABOX_S3_BUCKET:-deltabox-it}"
ACCESS_KEY="${DELTABOX_S3_ACCESS_KEY:-minioadmin}"
SECRET_KEY="${DELTABOX_S3_SECRET_KEY:-minioadmin}"
ENDPOINT="${DELTABOX_S3_ENDPOINT:-http://127.0.0.1:9000}"

if ! docker ps --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
  if docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER_NAME}"; then
    docker start "${CONTAINER_NAME}" >/dev/null
  else
    docker run -d \
      --name "${CONTAINER_NAME}" \
      -p 9000:9000 \
      -p 9001:9001 \
      -e "MINIO_ROOT_USER=${ACCESS_KEY}" \
      -e "MINIO_ROOT_PASSWORD=${SECRET_KEY}" \
      minio/minio server /data --console-address ":9001" >/dev/null
  fi
fi

READY_ATTEMPTS=0
until docker run --rm --network host --entrypoint /bin/sh minio/mc -c \
  "mc alias set local '${ENDPOINT}' '${ACCESS_KEY}' '${SECRET_KEY}' >/dev/null && mc ls local >/dev/null" >/dev/null 2>&1; do
  READY_ATTEMPTS=$((READY_ATTEMPTS + 1))
  if [ "${READY_ATTEMPTS}" -ge 60 ]; then
    echo "MinIO did not become ready with the configured credentials." >&2
    echo "Check ${CONTAINER_NAME} or set DELTABOX_S3_ACCESS_KEY / DELTABOX_S3_SECRET_KEY." >&2
    exit 1
  fi
  sleep 1
done

docker run --rm --network host --entrypoint /bin/sh minio/mc -c \
  "mc alias set local '${ENDPOINT}' '${ACCESS_KEY}' '${SECRET_KEY}' >/dev/null && mc mb --ignore-existing 'local/${BUCKET}' >/dev/null"

DELTABOX_RUN_MINIO_TESTS=1 \
DELTABOX_S3_ENDPOINT="${ENDPOINT}" \
DELTABOX_S3_BUCKET="${BUCKET}" \
DELTABOX_S3_ACCESS_KEY="${ACCESS_KEY}" \
DELTABOX_S3_SECRET_KEY="${SECRET_KEY}" \
cargo test -p deltabox-core --test minio_integration -- --ignored --nocapture
