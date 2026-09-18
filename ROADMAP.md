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
- DOCX 正文文本索引
- DOCX 页眉页脚文本索引
- XLSX 单元格文本索引
- PPTX 幻灯片文本索引
- PPTX speaker notes 索引
- 图片元数据索引，支持 EXIF / GPS
- 视频基础元数据索引
- Office 文档索引容错
- 搜索结果片段增强
- PDF 按页索引任务和可恢复进度
- UTF-8 文本按 chunk 索引任务和可恢复进度
- 索引任务 pause / resume 和 stale timeout
- CLI JSON 输出：`search --details --json`、`info --json`、`tag file --json`、`storage locations --json`
- `index segments <file_id> --json` 文本片段读取
- deltabox Skill 和本地 Agent 工作流验证
- `index_jobs` / `index_tasks` 可恢复索引任务模型
- 多 local backend
- storage copy / move / verify / locations
- 基础 replica policy 写入 manifest
- S3-compatible backend
- 真实 RustFS / S3 集成测试
- S3 backend 凭证本地加密保存
- H5 Web 应用：deltabox-server REST API + Vue 3 移动端界面（文件浏览、搜索、上传下载、标签、索引任务）

### 近期计划

1. **索引任务增强**
   - heartbeat
   - 视频按媒体区间拆分 task

2. **凭证保护升级**
   - vault password
   - OS keychain
   - recovery key
   - key rotation
   - backend credential migration

3. **远端入库与本地空间释放**
   - 添加文件时选择目标后端（S3 / 邮箱 / WebDAV）
   - 上传并逐 chunk 校验后可删除本地副本（offload），本地保留元数据和索引
   - 打开文件时按需从远端取回
   - 文件状态区分 local / remote_only / cached

4. **用量统计**
   - `usage_stats` 计数器表，数据变更时同事务增量维护
   - 逻辑量走计数器，物理量（SQLite 体积、目录占用）读取时实测
   - `stats` 命令和 `/api/stats`，含 `--reconcile` 对账校正
   - 区分 vault 总量与本机占用

### 中期计划

- 图片和扫描版 PDF 的 OCR
- Office 复杂内容索引：DOCX 批注/修订历史、XLSX 图表/公式/批注
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
- 基于共享后端的 vault 事件同步（event log over S3 / Email，无中心服务器）
  - vault_id、设备身份、事件签名
  - 共享后端上的事件日志为同步真相，本地 SQLite 为物化视图
  - 事件日志 compaction 与状态快照
  - 跨设备 GC 宽限期与冲突保留多版本
- 邮箱 backend 原型：IMAP APPEND 写入的 chunk 存储（按服务商 profile 配置 chunk 大小）+ `DeltaBox/Events` 事件同步通道（APPEND 优先、SMTP 兜底，可镜像到第二邮箱）
- 邮箱池（Email Pool）：多邮箱统一管理、健康状态机（healthy / degraded / unreachable）、degraded 时 rebalance、email → S3 迁移
- 移动端 App 原型（一等设备，持有自己的 vault 副本）
- MCP server 原型
  - 在 Skill 工作流和 CLI JSON 输出稳定后开始
  - 当需要多个智能体或应用共享结构化工具时开始
  - 当需要工具级权限、只读模式和 capability-based permission checks 时开始
  - 初始工具：`search_files`、`read_file_metadata`、`read_text_segments`、`tag_files`、`storage_locations`

### 长期计划

- 端到端加密的多设备同步完善（完整冲突解决、局域网发现、推送通知）
- 设备撤销与密钥轮换
- 分享与协作
- Local-first AI 助手集成
- 移动端 App 完整功能（后台索引、照片自动入库）
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
- DOCX body text indexing
- DOCX header/footer text indexing
- XLSX cell text indexing
- PPTX slide text indexing
- PPTX speaker notes indexing
- Image metadata indexing with EXIF / GPS
- Basic video metadata indexing
- Office indexing fault tolerance
- Detailed search result segments
- Page-level PDF indexing tasks and resumable progress
- Chunk-level UTF-8 text indexing tasks and resumable progress
- Index task pause / resume and stale timeout
- CLI JSON output: `search --details --json`, `info --json`, `tag file --json`, `storage locations --json`
- `index segments <file_id> --json` text segment reader
- deltabox Skill and local Agent workflow validation
- Recoverable indexing model with `index_jobs` / `index_tasks`
- Multiple local backends
- Storage copy / move / verify / locations
- Basic replica policy stored in manifests
- S3-compatible backend
- Real RustFS / S3 integration test
- Local encryption for S3 backend credentials
- H5 web app: deltabox-server REST API + Vue 3 mobile UI (file browsing, search, upload/download, tags, index jobs)

### Near-Term Plan

1. **Index Task Improvements**
   - Heartbeat
   - Media-range tasks for video

2. **Credential Protection Upgrade**
   - Vault password
   - OS keychain integration
   - Recovery key
   - Key rotation
   - Backend credential migration

3. **Remote Ingest and Local Space Reclaim**
   - Choose a target backend (S3 / email / WebDAV) when adding files
   - Optionally delete local chunks after per-chunk remote verification (offload), keeping metadata and indexes locally
   - Fetch files back from the remote on demand
   - Distinguish file states: local / remote_only / cached

4. **Usage Statistics**
   - `usage_stats` counter table updated in the same transaction as each mutation
   - Logical sizes via counters; physical sizes (SQLite file, directories) measured on read
   - `stats` command and `/api/stats`, with `--reconcile` for drift correction
   - Distinguish vault-wide totals from this-device usage

### Mid-Term Plan

- OCR for images and scanned PDFs
- Complex Office content indexing: DOCX comments/revisions, XLSX charts/formulas/comments
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
- Vault event sync over shared backends (event log over S3 / email, no central server)
  - vault_id, device identity, signed events
  - Event log on the shared backend is the sync truth; local SQLite is a materialized view
  - Event log compaction and state snapshots
  - Cross-device GC grace period and conflict-preserving versions
- Email backend prototype: IMAP APPEND-based chunk storage (per-provider chunk size profiles) + `DeltaBox/Events` sync channel (APPEND-first with SMTP fallback, optionally mirrored to a second mailbox)
- Email pool: unified management of multiple mailboxes, health states (healthy / degraded / unreachable), rebalance on degradation, and email → S3 migration
- Mobile app prototype (first-class device holding its own vault replica)
- MCP server prototype
  - Start after Skill workflows and CLI JSON output are stable
  - Start when multiple agents or apps need shared structured tools
  - Start when tool-level permissions, read-only mode, and capability-based permission checks are needed
  - Initial tools: `search_files`, `read_file_metadata`, `read_text_segments`, `tag_files`, `storage_locations`

### Long-Term Plan

- End-to-end encrypted multi-device sync hardening (full conflict resolution, LAN discovery, push notifications)
- Device revocation and key rotation
- Sharing and collaboration
- Local-first AI assistant integration
- Full mobile app features (background indexing, automatic photo ingest)
- Local vector index and semantic search
