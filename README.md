# deltabox

[中文](#中文) | [English](#english)

## 中文

deltabox 是一个 AI 增强型、去中心化优先的个人文件系统原型。它的目标不是做另一个中心化网盘，而是让用户可以把文件、索引、标签和存储位置掌握在自己手里，并为后续 Agent / MCP 集成做好准备。

当前仓库包含：

- `deltabox-core`：Rust 核心库，负责 manifest、chunk、storage backend、索引、标签、回收站和凭证保护。
- `deltabox-cli`：命令行原型，用于验证 core 的文件生命周期和存储迁移能力。

### 当前能力

- 本地 vault 初始化
- 文件导入、分片、hash、manifest 生成
- 本地 chunk storage backend
- 多 local backend 配置
- S3-compatible backend 配置与读写实现
- storage copy / move / verify / locations
- 真实 MinIO 集成测试，覆盖 S3 copy / verify / restore / move
- 回收站、恢复、永久删除和 chunk GC
- 用户标签创建、绑定、重命名、删除和标签搜索
- UTF-8 文本全文索引，基于 `text_segments` + SQLite FTS5
- 通用文本提取器抽象，为 PDF / Word / Excel 等格式索引做准备
- PDF text layer 索引，支持按页写入 `text_segments`
- 搜索结果片段增强，支持返回命中文本、页码和来源
- PDF 按页索引任务，支持 worker 分批处理和断点继续
- UTF-8 文本按 chunk 索引任务，支持大文本断点继续
- 索引任务 pause / resume 和 stale timeout
- CLI JSON 输出，支持 Agent 稳定解析 search/info/tag/storage 结果
- `index segments <file_id> --json`，支持按文件读取已索引文本片段
- 可恢复索引任务模型：`index_jobs` / `index_tasks`
- S3 access key / secret key 本地加密保存

### 快速开始

```bash
cargo build
cargo test
```

初始化 vault：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo init
```

添加文件：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo add ./notes.txt --path /docs/notes.txt
```

搜索文件名、路径、标签和全文内容：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning
```

查看搜索命中的片段、页码和来源：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning --details
```

输出 JSON 供 Agent / Skill 使用：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning --details --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo info <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index segments <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag file <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage locations <file_id> --json
```

控制后台索引任务：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index pause <job_id>
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index resume <job_id>
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index run --limit 10 --stale-timeout-seconds 600
```

添加标签：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag create 工作规划 --tag-type project
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag attach <file_id> 工作规划
```

添加本地备份 backend 并复制文件：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo backend add-local backup /tmp/deltabox-backup
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage copy <file_id> backup
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage locations <file_id>
```

添加 S3-compatible backend：

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo backend add-s3 minio \
  --endpoint http://localhost:9000 \
  --bucket deltabox \
  --region us-east-1 \
  --access-key <access_key> \
  --secret-key <secret_key> \
  --prefix chunks \
  --allow-http \
  --path-style true
```

运行本地 MinIO 集成测试：

```bash
scripts/minio-integration-test.sh
```

### 安全说明

当前版本会生成 `.deltabox/vault.key`，并用它加密保存 S3 backend 的 `access_key` 和 `secret_key`。这比明文存储更安全，但还不是最终的安全模型。后续计划接入用户密码、系统钥匙串、恢复密钥和密钥轮换。

### 项目状态

这是早期原型。CLI API、manifest schema 和数据库 schema 都可能继续变化。

## English

deltabox is an AI-enhanced, decentralization-first personal file system prototype. It is not intended to be another centralized cloud drive. The goal is to let users control their files, indexes, tags, and storage locations while preparing the system for future Agent and MCP integrations.

This repository contains:

- `deltabox-core`: the Rust core library for manifests, chunks, storage backends, indexes, tags, trash, and credential protection.
- `deltabox-cli`: a command-line prototype used to validate file lifecycle and storage migration behavior.

### Current Capabilities

- Local vault initialization
- File import, chunking, hashing, and manifest generation
- Local chunk storage backend
- Multiple local backend configuration
- S3-compatible backend configuration and implementation
- Storage copy / move / verify / locations
- Real MinIO integration test covering S3 copy / verify / restore / move
- Trash, restore, purge, and chunk GC
- User tag creation, attach, rename, delete, and tag search
- UTF-8 text full-text indexing with `text_segments` + SQLite FTS5
- Generic text extractor abstraction for future PDF / Word / Excel indexing
- PDF text layer indexing with page-level `text_segments`
- Detailed search results with matched text, page, and source
- Page-level PDF indexing tasks with resumable worker progress
- Chunk-level UTF-8 text indexing tasks with resumable progress
- Index task pause / resume and stale timeout
- CLI JSON output for stable Agent parsing of search/info/tag/storage results
- `index segments <file_id> --json` for reading indexed text segments by file
- Recoverable indexing task model with `index_jobs` / `index_tasks`
- Local encryption for S3 access key / secret key

### Quick Start

```bash
cargo build
cargo test
```

Initialize a vault:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo init
```

Add a file:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo add ./notes.txt --path /docs/notes.txt
```

Search by filename, path, tags, and indexed text:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning
```

Show matched segments, pages, and sources:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning --details
```

Output JSON for Agents / Skills:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo search planning --details --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo info <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index segments <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag file <file_id> --json
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage locations <file_id> --json
```

Control background indexing jobs:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index pause <job_id>
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index resume <job_id>
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo index run --limit 10 --stale-timeout-seconds 600
```

Add tags:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag create work-plan --tag-type project
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo tag attach <file_id> work-plan
```

Add a local backup backend and copy a file:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo backend add-local backup /tmp/deltabox-backup
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage copy <file_id> backup
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo storage locations <file_id>
```

Add an S3-compatible backend:

```bash
cargo run -p deltabox-cli -- --vault /tmp/deltabox-demo backend add-s3 minio \
  --endpoint http://localhost:9000 \
  --bucket deltabox \
  --region us-east-1 \
  --access-key <access_key> \
  --secret-key <secret_key> \
  --prefix chunks \
  --allow-http \
  --path-style true
```

Run the local MinIO integration test:

```bash
scripts/minio-integration-test.sh
```

### Security Note

The current version creates `.deltabox/vault.key` and uses it to encrypt S3 backend `access_key` and `secret_key` values. This is safer than plaintext storage, but it is not the final security model. Future work should add password-based vault unlock, OS keychain integration, recovery keys, and key rotation.

### Project Status

This is an early prototype. The CLI API, manifest schema, and database schema may continue to change.
