# deltabox Roadmap

[中文](#中文) | [English](#english)

## 中文

### 已完成

- Rust workspace：`deltabox-core` + `deltabox-cli`
- Vault 初始化和本地 metadata SQLite
- 文件导入、chunk hash、manifest、恢复
- 回收站、恢复、永久删除、chunk GC
- 用户标签系统和标签搜索
- UTF-8 文本文档全文索引
- 通用文本提取器抽象
- PDF text layer 索引，支持页码定位
- 搜索结果片段增强
- PDF 按页索引任务和可恢复进度
- 索引任务 pause / resume 和 stale timeout
- `index_jobs` / `index_tasks` 可恢复索引任务模型
- 多 local backend
- storage copy / move / verify / locations
- 基础 replica policy 写入 manifest
- S3-compatible backend
- 真实 MinIO / S3 集成测试
- S3 backend 凭证本地加密保存

### 近期计划

1. **索引任务增强**
   - heartbeat
   - 文本按 chunk 拆分 task
   - 视频按媒体区间拆分 task

2. **凭证保护升级**
   - vault password
   - OS keychain
   - recovery key
   - key rotation
   - backend credential migration

3. **MCP server 原型**
   - `search_files`
   - `read_file_metadata`
   - `read_text_segments`
   - `tag_files`
   - `storage_locations`
   - capability-based permission checks

### 中期计划

- 图片和扫描版 PDF 的 OCR
- EXIF / GPS 元数据提取
- 视频关键帧和语音转文字索引
- 后台 worker 守护进程
- 本地桌面应用原型
- 大文件跨 backend 分布式存储
  - 按 chunk 将单个文件分布到多个 backend
  - 支持按容量、成本、可用性和用户策略选择 backend
  - restore 时从多个 backend 拉取 chunk 并重组文件
  - 后续支持并行上传/下载
- 存储策略再平衡器
- S3-compatible backend 稳定性增强
- WebDAV backend
- 邮箱 backend 原型

### 长期计划

- 端到端加密的多设备同步
- 设备身份和设备撤销
- 分享与协作
- Local-first AI 助手集成
- 面向 Codex / Claude Code 的 Agent Skill
- 支持可恢复后台索引的移动端应用
- 本地向量索引和语义搜索

## English

### Completed

- Rust workspace: `deltabox-core` + `deltabox-cli`
- Vault initialization and local metadata SQLite
- File import, chunk hash, manifest, and restore
- Trash, restore, purge, and chunk GC
- User tag system and tag search
- Full-text indexing for UTF-8 text documents
- Generic text extractor abstraction
- PDF text layer indexing with page locators
- Detailed search result segments
- Page-level PDF indexing tasks and resumable progress
- Index task pause / resume and stale timeout
- Recoverable indexing model with `index_jobs` / `index_tasks`
- Multiple local backends
- Storage copy / move / verify / locations
- Basic replica policy stored in manifests
- S3-compatible backend
- Real MinIO / S3 integration test
- Local encryption for S3 backend credentials

### Near-Term Plan

1. **Index Task Improvements**
   - Heartbeat
   - Chunk-level tasks for text files
   - Media-range tasks for video

2. **Credential Protection Upgrade**
   - Vault password
   - OS keychain integration
   - Recovery key
   - Key rotation
   - Backend credential migration

3. **MCP Server Prototype**
   - `search_files`
   - `read_file_metadata`
   - `read_text_segments`
   - `tag_files`
   - `storage_locations`
   - Capability-based permission checks

### Mid-Term Plan

- OCR for images and scanned PDFs
- EXIF / GPS metadata extraction
- Video keyframe and ASR text indexing
- Background worker daemon
- Local desktop app prototype
- Large-file cross-backend distributed storage
  - Distribute chunks of one file across multiple backends
  - Choose backends by capacity, cost, availability, and user policy
  - Restore files by reading chunks from multiple backends
  - Add parallel upload/download later
- Storage policy rebalancer
- S3-compatible backend hardening
- WebDAV backend
- Email backend prototype

### Long-Term Plan

- End-to-end encrypted multi-device sync
- Device identity and revocation
- Sharing and collaboration
- Local-first AI assistant integration
- Agent Skills for Codex / Claude Code
- Mobile app with resumable background indexing
- Local vector index and semantic search
