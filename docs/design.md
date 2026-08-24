# deltabox 设计文档

## 1. 项目定位

deltabox 是一个 AI 增强型去中心化网盘软件。

它的核心目标不是再做一个中心化云盘，而是为用户建立一个可长期拥有、可迁移、可搜索、可被智能体安全调用的个人文件系统。

deltabox 的基础能力在没有大语言模型时也必须可用，包括文件上传、同步、加密、备份、搜索和恢复。LLM 是增强层，用来理解自然语言意图、规划复杂查询、总结文件内容、执行自动整理等任务。

简化定位：

```text
去中心化个人文件系统 + 智能索引 + 可选 AI 智能体
```

## 2. 设计原则

### 2.1 本地优先

文件、索引、权限、密钥应优先由用户设备掌控。云端存储只作为数据承载介质，不作为唯一真相。

### 2.2 存储后端可替换

邮箱、S3、OSS、WebDAV、第三方网盘、本地目录、NAS 都应被抽象成存储后端。用户可以使用一种或多种后端组合。

### 2.3 AI 增强而非 AI 强依赖

无 LLM 时，deltabox 是一个普通但可靠的去中心化网盘。

接入 LLM 后，deltabox 成为可以理解自然语言、调用工具、协助整理和检索文件的智能体网盘。

### 2.4 索引可重建

原始文件、加密分片、manifest 和密钥是真相。搜索索引、向量索引、缩略图、OCR 结果都应视为可重建资产。

### 2.5 隐私默认保护

默认不把文件内容上传给第三方模型。所有 AI 能力应支持本地优先策略，用户可以明确选择是否接入云端模型。

### 2.6 面向智能体的系统边界

从第一天开始，文件搜索、读取、移动、标签、分享等能力都应有清晰的工具接口，方便未来被智能体安全调用。

## 3. 目标用户

### 3.1 个人用户

需要长期保存照片、文档、票据、证件、笔记和项目资料，希望数据不被单一云服务绑定。

### 3.2 知识工作者

需要在大量文档、截图、照片、会议资料、项目文件中快速找到相关内容。

### 3.3 隐私敏感用户

希望文件和索引尽量保留在本地或自有存储中，不默认暴露给中心化平台。

### 3.4 AI 工具用户

希望通过自然语言完成文件查找、总结、整理、归档、分享等任务。

## 4. 核心使用场景

### 4.1 普通文件同步

用户把文件放入 deltabox，系统自动完成分片、加密、上传、索引和多设备同步。

### 4.2 多后端备份

用户可以把重要文件同时存储到多个后端：

```text
照片 -> 本地 NAS + S3
文档 -> S3 + 邮箱
证件 -> 本地目录 + 加密备份
```

### 4.3 自然语言查找

用户输入：

```text
帮我找出去年我在图书馆的照片
```

系统应理解为：

```text
文件类型：图片
时间范围：上一自然年
地点：图书馆
匹配依据：EXIF GPS、地点标签、OCR、图片语义标签、文件夹名
```

### 4.4 文档语义检索

用户输入：

```text
帮我找一下我今年工作规划的文件
```

系统应检索：

```text
文件名：工作规划、年度计划、OKR、目标、路线图
正文内容：文档、PDF、表格、笔记
OCR 内容：截图、白板照片、扫描件
时间范围：今年
```

### 4.5 智能整理

用户输入：

```text
把最近三个月的发票整理到报销文件夹
```

系统应先检索候选文件，展示将要执行的操作，再等待用户确认。

### 4.6 文件总结

用户输入：

```text
总结这个项目文件夹里最重要的资料
```

系统读取授权范围内的文件，生成摘要、关键文件列表和可能的缺失资料。

### 4.7 用户自定义标签

用户可以为文件、人物、地点、项目或文件夹打标签。

示例：

```text
人物：大儿子、小女儿、张三
地点：公司会议室、老家、上海图书馆
项目：deltabox、客户 A、Q1 规划
类别：发票、合同、证件、医疗
```

用户标签的可信度高于模型标签。模型可以建议标签，但涉及人物、地点、敏感类别时，应优先由用户确认。

标签不是只在文件存入时设置。用户应能在任何时候新增、修改、删除、合并或重命名标签，系统需要同步更新搜索索引和相关规则。

## 5. 总体架构

```text
Client Apps
  Desktop / Mobile / CLI / Web

Agent Layer
  Intent Parser
  Query Planner
  Tool Runtime
  Permission Guard
  Model Provider

Search & Index Layer
  Metadata DB
  Full-text Search
  OCR Index
  Vector Index
  Media Analyzer
  Document Parser

Core File Layer
  Manifest Store
  Chunk Manager
  Version Manager
  Sync Engine
  Conflict Resolver

Crypto Layer
  Key Management
  Content Encryption
  Signature
  Device Identity

Storage Adapter Layer
  Local
  S3 / OSS
  Email / IMAP / SMTP
  WebDAV
  Third-party Drive
  NAS
```

## 6. 模块设计

### 6.1 Client Apps

客户端负责用户交互和本地数据管理。

主要能力：

- 文件浏览
- 上传下载
- 搜索
- 自然语言输入
- 后端配置
- 密钥管理
- 同步状态展示
- 冲突处理
- 权限确认

首批客户端建议：

- Desktop：优先开发，适合处理大量文件和完整索引
- CLI：方便调试核心能力
- Mobile：后续支持照片自动备份和移动端检索

### 6.2 Core File Layer

核心文件层负责把文件变成可同步、可校验、可恢复的数据结构。

主要职责：

- 文件分片
- 内容 hash
- chunk 去重
- manifest 生成
- 版本管理
- 删除标记
- 多后端位置记录
- 文件恢复

文件不应直接绑定某个远端对象。一个文件应由 manifest 描述，内容由多个 chunk 组成，chunk 可以存放在多个后端。

### 6.3 Storage Adapter Layer

所有存储服务都通过统一接口接入。

概念接口：

```rust
trait StorageBackend {
    async fn put_chunk(&self, chunk_id: ChunkId, data: &[u8]) -> Result<()>;
    async fn get_chunk(&self, chunk_id: ChunkId) -> Result<Vec<u8>>;
    async fn has_chunk(&self, chunk_id: ChunkId) -> Result<bool>;
    async fn delete_chunk(&self, chunk_id: ChunkId) -> Result<()>;
    async fn list_chunks(&self) -> Result<Vec<ChunkId>>;
}
```

首批后端优先级：

1. Local directory
2. S3 compatible storage
3. OSS
4. Email / IMAP / SMTP
5. WebDAV

### 6.4 Storage Migration and Rebalancing

deltabox 必须支持文件在不同存储后端之间迁移。

典型场景：

```text
本地硬盘空间不足 -> 迁移到 S3 / OSS
本地 NAS 损坏风险升高 -> 增加云端副本
某个云服务即将停用 -> 迁移到另一个云服务
用户从临时后端切换到长期后端 -> 批量重定位 chunk
移动设备只保留缓存 -> 完整数据迁移到桌面端或云端
```

迁移不应改变文件 ID、文件版本和用户看到的文件路径。迁移只改变 chunk 的存储位置。

文件 manifest 中的 `locations` 字段用于描述同一个 chunk 存在哪些后端：

```json
{
  "chunk_id": "sha256:...",
  "size": 1048576,
  "locations": [
    {
      "backend_id": "local_disk",
      "status": "available",
      "object_key": "chunks/ab/cd/..."
    },
    {
      "backend_id": "s3_primary",
      "status": "available",
      "object_key": "chunks/ab/cd/..."
    }
  ]
}
```

迁移流程：

```text
1. 选择源后端和目标后端
2. 枚举需要迁移的 chunk
3. 从源后端读取加密 chunk
4. 校验 chunk hash
5. 写入目标后端
6. 从目标后端读回或校验存在性
7. 更新 manifest 的 locations
8. 达到副本策略后，可选择删除旧后端副本
```

迁移必须是可中断、可恢复、可校验的后台任务。

迁移任务状态示例：

```json
{
  "job_id": "job_01J...",
  "type": "storage_migration",
  "source_backend": "local_disk",
  "target_backend": "s3_primary",
  "scope": {
    "folder_id": "folder_photos",
    "include_versions": true
  },
  "status": "running",
  "total_chunks": 12000,
  "completed_chunks": 8300,
  "failed_chunks": 2
}
```

迁移策略：

```text
copy_then_verify
  先复制，校验成功后再允许删除源副本。默认策略。

mirror
  保留源和目标两个副本，用于重要文件冗余。

move_after_threshold
  目标后端稳定保存一段时间后，自动删除源副本。

cache_only_local
  本地只保留最近使用文件，完整数据保存在网络后端。
```

用户界面应提供：

- 后端容量和健康状态
- 文件或文件夹的存储位置
- 批量迁移入口
- 迁移进度
- 失败重试
- 删除旧副本前的确认
- 本地缓存大小限制

重要约束：

- 删除旧副本前必须确认目标副本可用
- manifest 更新必须具备事务性
- 迁移过程中用户仍应能读取文件
- 若源后端已经不可用，只能迁移仍可从其他 location 找到的 chunk
- 如果某些 chunk 只有一个已失效 location，系统应标记文件为 `incomplete`

### 6.5 Replica Policy

多处备份不应是强制行为。

deltabox 应支持按文件、文件夹、标签、文件类型或存储后端设置副本策略。系统默认可以推荐安全策略，但最终由用户决定成本、可靠性和访问速度之间的取舍。

副本策略示例：

```text
single_copy
  只保存一份完整数据。适合大视频、临时文件、可重新下载的资料。

local_primary_cloud_backup
  本地为主，云端备份。适合常用文档和照片。

cloud_primary_local_cache
  云端保存完整数据，本地只保留最近使用文件。适合移动设备和小硬盘电脑。

mirror_two_backends
  两个后端各保存一份。适合重要文档、证件、家庭照片。

metadata_only_local
  本地只保存 metadata、索引和缩略图，原文件按需下载。

no_backup
  不纳入备份，只做本地管理。适合临时文件。
```

manifest 中记录的是实际位置，replica policy 记录的是期望状态。

示例：

```json
{
  "file_id": "file_...",
  "replica_policy": {
    "mode": "cloud_primary_local_cache",
    "min_full_copies": 1,
    "preferred_backends": ["s3_primary"],
    "cache_backends": ["local_disk"],
    "local_cache_ttl_days": 30
  }
}
```

后台的 rebalancer 负责检查实际状态是否满足期望状态：

```text
期望：至少 1 份完整副本在 S3
实际：只有本地副本
动作：上传到 S3，成功后可按策略删除或保留本地副本
```

对于大文件，默认策略应更保守：

```text
大视频
  默认 single_copy 或 cloud_primary_local_cache

照片
  默认 local_primary_cloud_backup，可由用户改为 single_copy

证件/合同
  默认 mirror_two_backends 或提示用户开启第二副本

临时下载
  默认 no_backup 或 single_copy
```

用户界面应明确显示：

- 当前文件有几份完整副本
- 每份副本在哪个后端
- 是否满足当前副本策略
- 预计额外占用空间
- 删除某个副本后的风险

重要原则：

- 多副本是可靠性能力，不是强制成本
- 默认策略应避免让大文件意外产生多份昂贵副本
- 删除唯一副本前必须警告
- 对重要文件可提示用户增加副本，但不应自动强制
- 本地缓存不等于完整副本，界面必须区分

### 6.6 Email Backend

邮箱后端借鉴 Delta Chat 的思想：复用已有邮箱基础设施，而不是依赖 deltabox 官方服务器。

设计方式：

- chunk 作为 MIME 附件保存
- manifest 作为小型 JSON 文档保存
- 使用专门邮箱文件夹，例如 `DeltaBox/Chunks` 和 `DeltaBox/Manifest`
- SMTP 负责写入
- IMAP 负责读取和同步
- 邮件 header 标记 deltabox 数据类型

示例 header：

```text
X-DeltaBox-Type: chunk
X-DeltaBox-Chunk-Id: sha256:...
X-DeltaBox-File-Id: file_...
X-DeltaBox-Version: 1
```

限制：

- 邮箱附件大小有限
- 服务商可能限流
- IMAP 搜索不稳定
- 不适合高频大文件写入
- 更适合作为低频备份或冗余后端

### 6.7 Crypto Layer

加密层应确保后端无法直接读取用户文件。

基础要求：

- 客户端加密
- 每个 chunk 加密后上传
- manifest 可选择加密
- 文件名可选择加密或半隐藏
- 设备身份签名
- 多设备密钥同步

建议模型：

```text
Master Key
  -> Device Key
  -> File Key
  -> Chunk Encryption Key
```

需要支持：

- 新设备加入
- 设备撤销
- 密钥备份
- 恢复码
- 共享文件的独立密钥

### 6.8 Manifest Store

manifest 描述文件逻辑状态，是系统的核心真相之一。

示例：

```json
{
  "file_id": "file_01HZY...",
  "name": "2026_work_plan.pdf",
  "mime": "application/pdf",
  "size": 2389120,
  "created_at": "2026-01-15T09:20:00Z",
  "modified_at": "2026-01-20T14:10:00Z",
  "content_hash": "sha256:...",
  "version": 3,
  "chunks": [
    {
      "chunk_id": "sha256:...",
      "offset": 0,
      "size": 1048576,
      "locations": [
        {
          "backend_id": "s3_primary",
          "object_key": "chunks/ab/cd/..."
        }
      ]
    }
  ],
  "metadata": {
    "tags": ["work", "planning"],
    "source": "desktop",
    "index_state": "indexed"
  }
}
```

### 6.9 Sync Engine

同步引擎负责多设备和多后端一致性。

核心策略：

- 内容不可变，chunk 以 hash 标识
- 文件状态通过 manifest event log 更新
- 删除采用 tombstone
- 冲突保留多版本
- 索引不作为同步真相

事件示例：

```json
{
  "event_id": "evt_01HZZ...",
  "type": "file.updated",
  "file_id": "file_01HZY...",
  "version": 4,
  "device_id": "device_macbook",
  "created_at": "2026-07-09T10:00:00Z",
  "manifest_hash": "sha256:..."
}
```

### 6.10 删除、回收站与清理

deltabox 应支持删除文件到回收站，而不是默认立即物理删除。

删除需要区分三个层次：

```text
move_to_trash
  用户删除文件时，先把文件从正常视图移入回收站。

restore_from_trash
  用户可以在保留期内恢复文件。

purge
  用户明确永久删除，或超过保留期后由系统清理。
```

回收站本质上是 manifest 状态变化，不应立即删除 chunk。

示例：

```json
{
  "event_id": "evt_01J...",
  "type": "file.trashed",
  "file_id": "file_01HZY...",
  "previous_path": "/work/plan.pdf",
  "trashed_at": "2026-07-09T12:30:00Z",
  "device_id": "device_macbook"
}
```

文件状态：

```text
active
  正常文件。

trashed
  已进入回收站，默认不出现在普通文件列表，但可搜索回收站。

purged
  已永久删除，manifest 只保留 tombstone 或审计记录。

incomplete
  manifest 存在，但部分 chunk 不可用。
```

恢复流程：

```text
1. 用户从回收站选择恢复
2. 系统检查原路径是否可用
3. 如果原路径冲突，生成新名称或要求用户选择
4. 写入 file.restored 事件
5. 文件重新进入 active 状态
```

永久删除流程：

```text
1. 用户确认永久删除，或保留期到期
2. 写入 file.purged 事件
3. manifest 标记为 purged
4. 后台 GC 检查 chunk 是否仍被其他文件或历史版本引用
5. 仅删除无人引用的 chunk location
```

chunk 不能在文件进入回收站时立即删除，因为：

- 用户可能恢复文件
- 其他版本可能引用相同 chunk
- 其他文件可能因为去重引用相同 chunk
- 其他设备可能还没有收到删除事件

回收站策略：

```text
retention_days
  默认保留 30 天，可由用户修改。

trash_quota
  回收站最大占用空间，超过后提示清理。

per_folder_policy
  某些文件夹可设置更长或更短保留期。

sensitive_policy
  敏感文件可选择删除时立即要求永久删除确认。
```

多设备行为：

```text
设备 A 删除文件到回收站
设备 B 收到 file.trashed 事件后，从普通视图隐藏该文件
设备 B 仍可在回收站中看到该文件
任一有权限设备都可以恢复或永久删除
```

删除冲突：

```text
设备 A 删除文件
设备 B 离线修改同一文件
同步后系统保留删除事件和修改事件
默认生成冲突版本，避免丢失 B 的修改
```

用户界面应显示：

- 回收站入口
- 删除时间
- 原始路径
- 文件大小
- 占用的唯一 chunk 空间
- 预计自动清理时间
- 恢复和永久删除操作

## 7. 索引系统

### 7.1 索引分层

索引库会随着文件增长而变大，因此必须分层。

```text
L0 必需索引
  文件 ID、名称、大小、hash、时间、类型、存储位置

L1 搜索索引
  文件名、标签、EXIF、OCR、文档正文、全文倒排索引

L2 智能索引
  embedding、图片 caption、人物、场景、物体、语义标签

L3 缓存
  缩略图、预览图、临时解析结果
```

L0 必须可靠同步。L1 可以同步或重建。L2 默认本地，可选择同步。L3 完全可删除。

### 7.2 索引时机

deltabox 不应等到用户搜索时才识别所有文件，也不应在入库时强制完成所有重型分析。

推荐采用混合策略：

```text
入库同步阶段
  立即执行 hash、manifest、文件名、类型、大小、时间、EXIF、已有元数据提取。

快速索引阶段
  尽快执行文本提取、OCR、PDF/docx 正文解析、基础标签生成。

后台增强阶段
  在设备空闲、充电或用户允许时执行图片语义标签、人脸聚类、视频关键帧、ASR、embedding。

搜索时按需阶段
  对未索引、索引过期或低置信度的候选文件补做分析。
```

不同文件类型的默认策略：

```text
照片
  入库：EXIF、GPS、尺寸、缩略图
  后台：OCR、人脸检测、场景/物体标签、embedding
  搜索时：对缺失 OCR 或低置信度候选图片补识别

文档
  入库：文件名、类型、大小、时间
  快速索引：正文抽取、标题、前几页文本、全文索引
  后台：摘要、关键词、embedding

视频
  入库：时长、分辨率、拍摄时间、GPS、缩略图
  后台：关键帧、关键帧 OCR、视觉标签、音频 ASR、字幕提取
  搜索时：只对候选视频补充抽帧或转写
```

产品上应提供索引模式：

```text
省电省空间
  只做基础索引，搜索时按需分析。

均衡模式
  入库做基础索引，后台做 OCR 和文档全文，重型视频分析延后。

智能增强模式
  后台生成 OCR、embedding、图片标签、人脸聚类、视频片段索引。
```

### 7.3 元数据索引

保存基础结构化信息：

- 文件名
- 扩展名
- MIME 类型
- 大小
- 创建时间
- 修改时间
- 拍摄时间
- 导入时间
- 设备来源
- 存储后端
- 标签
- 文件夹

建议使用 SQLite 作为本地 metadata database。

### 7.4 用户标签索引

用户标签是 deltabox 的一等索引数据。

用户标签适合描述机器无法可靠推断的个人语境：

```text
大儿子
公司会议室
老家
客户 A
年度复盘
重要合同
```

标签类型：

```text
普通标签
  发票、合同、旅行、工作规划

人物标签
  大儿子、小女儿、张三

地点标签
  公司会议室、上海图书馆、老家

项目标签
  deltabox、Q1 计划、客户 A

敏感标签
  证件、财务、医疗、私密
```

标签数据应记录来源和置信度：

```json
{
  "file_id": "file_123",
  "tags": [
    {
      "name": "大儿子",
      "type": "person",
      "source": "user",
      "confidence": 1.0
    },
    {
      "name": "child",
      "type": "visual",
      "source": "model",
      "confidence": 0.82
    },
    {
      "name": "公司会议室",
      "type": "place",
      "source": "user_rule",
      "confidence": 0.95
    }
  ]
}
```

产品能力：

- 单文件打标签
- 批量打标签
- 搜索结果中直接打标签
- 后续新增标签
- 修改标签名称和类型
- 从文件移除标签
- 删除不再使用的标签
- 合并重复标签
- 标签重命名后自动更新相关文件
- 人脸分组命名
- 地点别名命名
- 文件夹默认标签
- 自动标签建议
- 标签规则
- 敏感标签策略
- 标签导入导出

标签规则示例：

```text
如果 GPS 在公司附近，建议标签：公司
如果 Wi-Fi 为公司网络，建议标签：公司会议室
如果 OCR 包含“增值税发票”，建议标签：发票
如果人脸聚类 face_cluster_123 被用户命名为“大儿子”，后续相同聚类建议标签：大儿子
```

规则生成的标签应可回滚，人物、地点和敏感标签默认应进入待确认状态。

标签生命周期：

```text
create
  用户新增标签，例如“大儿子”。

attach
  将标签绑定到一个或多个文件、文件夹、人脸聚类或地点。

update
  修改标签名称、类型、颜色、隐私级别或说明。

detach
  从某些文件上移除标签，但保留标签本身。

merge
  合并重复标签，例如“公司会议室”和“会议室-公司”。

delete
  删除标签，并从相关文件索引中移除。

reindex
  标签变化后更新 metadata、全文索引、搜索排序和 Agent 可见上下文。
```

标签操作应写入事件日志，便于多设备同步、撤销和冲突解决。

示例事件：

```json
{
  "event_id": "evt_tag_01J...",
  "type": "tag.updated",
  "tag_id": "tag_big_son",
  "changes": {
    "name": ["儿子", "大儿子"],
    "type": ["generic", "person"]
  },
  "device_id": "device_macbook",
  "created_at": "2026-07-09T12:00:00Z"
}
```

标签修改后的影响：

```text
搜索
  新标签或改名后的标签应立即可搜。

智能体
  Agent 查询计划应使用最新标签名称和别名。

规则
  标签规则引用的标签被重命名或合并时，应自动迁移引用。

索引
  标签更新不应要求重新上传文件，只需要更新 metadata 和搜索索引。
```

### 7.5 OCR 索引

OCR 是 deltabox 的关键能力之一。

适用对象：

- 照片
- 截图
- 扫描件
- PDF 图片页
- 白板照片
- 票据
- 海报

OCR 结果应保存：

```json
{
  "file_id": "file_...",
  "language": "zh-CN",
  "text": "2026 年工作规划",
  "blocks": [
    {
      "text": "2026 年工作规划",
      "bbox": [120, 80, 640, 150],
      "confidence": 0.94
    }
  ]
}
```

实现建议：

- iOS/macOS：Apple Vision
- Android：ML Kit Text Recognition
- Desktop：PaddleOCR 或 Tesseract
- Server optional：用户自建 OCR 服务

### 7.6 文档索引

支持格式：

- txt
- md
- pdf
- docx
- xlsx
- pptx
- html
- email

索引内容：

- 标题
- 正文
- 表格文本
- 页码
- 摘要
- 关键段落

对大文档应支持分页索引和摘要索引，避免索引库无限膨胀。

### 7.7 图片语义索引

图片不只有 OCR，还应支持视觉语义。

可索引内容：

- 场景：图书馆、办公室、会议室
- 物体：电脑、白板、书、发票
- 活动：会议、演讲、旅行
- 地点：GPS、地标、用户标签
- 人物：本地人脸聚类，默认不上传

图片语义索引可以先作为可选增强能力，不作为 MVP 必需项。

无 LLM 时，图片语义可以通过专用视觉模型实现：

```text
人脸检测
人脸聚类
年龄/性别粗分类
物体识别
场景分类
图片 embedding
```

注意事项：

- 人脸聚类可以识别“同一个人”，但不应自动推断真实身份
- “大儿子”“客户 A”等身份应来自用户命名
- 年龄、性别等结果只能作为低置信度辅助信号
- 人脸和人物标签默认本地处理，不上传第三方服务

### 7.8 视频索引和摘要

视频内容索引应采用分层流水线，不应默认把完整视频发送给多模态 LLM。

基础流水线：

```text
1. 提取元数据
   时长、分辨率、拍摄时间、GPS、设备、编码信息。

2. 场景切分
   根据画面变化把视频切成多个片段。

3. 抽关键帧
   每个片段取 1-3 张代表帧，或按固定间隔抽帧。

4. 关键帧 OCR
   提取字幕、PPT、白板、屏幕文字。

5. 音频 ASR
   把语音转成带时间戳的文本。

6. 视觉标签
   对关键帧识别场景、人物、物体、动作。

7. 片段索引
   保存时间范围、OCR、ASR、视觉标签、缩略图、embedding。
```

片段索引示例：

```json
{
  "file_id": "video_123",
  "duration": 1840,
  "segments": [
    {
      "start": "00:02:10",
      "end": "00:05:40",
      "asr_text": "今年的工作规划主要分三部分...",
      "ocr_text": "2026 年度目标 / Q1 OKR",
      "visual_labels": ["meeting", "presentation", "whiteboard"],
      "keyframes": ["kf_001.jpg", "kf_002.jpg"]
    }
  ]
}
```

无 LLM 时，可以生成结构化摘要：

```text
视频时长 30 分钟，主要包含会议、演示、白板内容。
识别到关键词：2026 年度目标、Q1 OKR、预算、项目排期。
主要语音内容集中在：工作规划、团队分工、交付时间。
```

接入文本 LLM 后，优先把 ASR、OCR、关键帧 caption、视觉标签和时间轴片段交给模型生成自然语言摘要。

多模态 LLM 适合以下场景：

```text
短视频摘要
画面信息比语音更重要
没有语音或字幕
需要理解复杂画面关系
用户明确询问“这段视频里发生了什么”
```

默认策略：

```text
无 LLM
  生成结构化摘要、关键词、时间轴、片段标签。

文本 LLM
  基于 ASR/OCR/标签生成自然语言摘要。

多模态 LLM
  只对短视频、关键片段或用户明确请求的片段使用。
```

### 7.9 向量索引

向量索引用于语义搜索。

示例：

```text
查询：今年工作规划
可匹配：年度目标、OKR、路线图、Q1 plan、战略安排
```

建议：

- 默认本地生成 embedding
- 支持用户选择模型
- 手机端可只保留轻量向量索引
- 大型向量索引优先放在桌面端

可选技术：

- sqlite-vec
- LanceDB
- Qdrant embedded
- Tantivy + vector extension

## 8. 搜索系统

### 8.1 查询类型

搜索系统应支持三种查询：

```text
结构化查询
  时间、类型、大小、地点、标签、后端

全文查询
  文件名、OCR、文档正文、注释、标签

语义查询
  embedding、图片 caption、摘要、相似文件
```

### 8.2 查询计划

自然语言查询不应直接交给文件系统执行，而应先转换成查询计划。

示例：

用户输入：

```text
帮我找出去年我在图书馆的照片
```

查询计划：

```json
{
  "intent": "find_files",
  "filters": {
    "mime_group": "image",
    "time_range": {
      "from": "2025-01-01T00:00:00Z",
      "to": "2025-12-31T23:59:59Z"
    },
    "location_hint": "图书馆"
  },
  "signals": [
    "exif_gps",
    "user_tags",
    "ocr_text",
    "visual_labels",
    "folder_path"
  ],
  "sort": "relevance"
}
```

### 8.3 结果解释

搜索结果应解释为什么匹配。

示例：

```text
library_2025_03_18.jpg
匹配原因：
- 拍摄时间：2025-03-18
- GPS 附近地点：上海图书馆
- 图片 OCR：借阅区
- 图片类型：照片
```

这对用户信任和智能体调试都很重要。

## 9. AI Agent 设计

### 9.1 Agent 定位

Agent 不直接拥有文件系统权限。它只能通过受控工具访问 deltabox。

Agent 的职责：

- 理解用户自然语言
- 生成查询计划
- 调用搜索工具
- 读取必要文件摘要
- 请求用户确认
- 执行整理、移动、标签、分享等操作

### 9.2 工具接口

基础工具：

```text
search_files
read_file_metadata
read_file_text
read_file_preview
list_folder
tag_files
move_files
copy_files
share_files
summarize_files
```

危险工具需要确认：

```text
delete_files
overwrite_files
share_publicly
send_to_external_model
export_decrypted_archive
```

### 9.3 权限保护

Agent 每次调用工具都应经过权限检查。

权限维度：

- 文件范围
- 文件类型
- 时间范围
- 是否允许读取正文
- 是否允许读取原文件
- 是否允许发送到模型
- 是否允许修改
- 是否允许分享

### 9.4 模型接入

Model Provider 应作为可替换组件。

支持类型：

- 本地模型
- OpenAI compatible API
- 企业私有模型
- 用户自定义 HTTP endpoint

接口概念：

```rust
trait ModelProvider {
    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse>;
    async fn embed(&self, input: EmbedInput) -> Result<Embedding>;
}
```

### 9.5 无 LLM 降级

无 LLM 时，自然语言框仍可做基础解析。

例如：

```text
图书馆 2025 照片
```

可降级成：

```text
关键词：图书馆
时间：2025
类型：图片
```

无 LLM 不等于没有 AI。deltabox 可以继续使用本地或平台提供的专用模型：

```text
OCR
  识别照片、截图、扫描件、视频关键帧中的文字。

ASR
  把视频和音频中的语音转成文本。

视觉模型
  识别物体、场景、人脸聚类、图片相似度。

Embedding 模型
  支持文本、图片、音频的相似搜索和语义检索。
```

LLM 的主要价值在于：

```text
理解模糊自然语言
组合多个查询条件
解释复杂结果
总结长文档或长视频
规划多步骤文件操作
```

因此，无 LLM 模式应定位为：

```text
结构化索引 + 专用模型识别 + 搜索引擎
```

有 LLM 模式应定位为：

```text
在索引和工具之上增加自然语言理解、推理、总结和智能体操作
```

## 10. 数据安全和隐私

### 10.1 默认策略

- 文件默认客户端加密
- 索引默认本地保存
- embedding 默认本地生成
- 缩略图默认本地缓存
- 云端模型默认不读取原文
- 敏感文件可排除出智能索引

### 10.2 敏感文件模式

用户可对文件或文件夹设置：

```text
不生成 OCR
不生成 embedding
不生成摘要
不允许 Agent 读取正文
不允许发送给外部模型
只允许精确文件名搜索
```

### 10.3 外部模型确认

任何把文件内容、OCR 文本、摘要、embedding 输入发送给第三方模型的操作都应有明确提示。

提示应说明：

- 将发送哪些文件
- 发送哪些内容
- 发送到哪个模型服务
- 是否保存对话记录
- 是否可撤销

## 11. 多设备设计

### 11.1 设备身份

每台设备拥有独立设备 ID 和设备密钥。

设备加入流程应包含：

- 老设备确认
- 扫码或恢复码
- 密钥传递
- 权限范围选择

### 11.2 设备能力差异

不同设备可保留不同索引级别：

```text
桌面端
  完整索引、全文、OCR、向量、缩略图

手机端
  基础索引、最近文件、轻量 OCR、按需下载

服务器/NAS
  后台同步、备份、重建索引、批量 OCR
```

### 11.3 分布式操作与近实时同步

deltabox 应支持多设备分布式操作。用户在手机、桌面端或 NAS 上进行的文件操作，应通过事件日志传播到其他设备。

典型场景：

```text
手机上传一张照片
  -> 手机生成 manifest 和 chunk
  -> 上传 chunk 到可用后端
  -> 写入 file.created 事件
  -> 桌面端收到事件
  -> 桌面端更新文件列表和索引
  -> 桌面端按需下载原图或缩略图
```

文件操作不应依赖某个中心服务器才能成立。每台设备都可以先在本地提交操作，再通过共享后端、P2P 连接或可选中继服务传播。

基础流程：

```text
1. Local Commit
   设备先在本地数据库写入操作事件。

2. Chunk Upload
   如果操作涉及新文件或新版本，先上传加密 chunk。

3. Manifest Publish
   发布新的 manifest 或 manifest event。

4. Event Propagation
   通过存储后端、P2P、局域网发现、推送通知或可选中继服务通知其他设备。

5. Remote Apply
   其他设备拉取事件，校验签名，更新本地状态。

6. Lazy Fetch
   其他设备先显示 metadata 和缩略图，原文件按需下载。
```

事件类型：

```text
file.created
file.updated
file.deleted
file.moved
file.renamed
tag.attached
tag.detached
tag.updated
folder.created
backend.added
device.added
device.revoked
share.created
```

近实时同步可以分成多种通道：

```text
local_network
  同一局域网内设备直接发现和同步，速度最快。

backend_polling
  通过 S3、邮箱、WebDAV 等共享后端轮询 event log，可靠但可能有延迟。

push_notification
  移动端通过可选推送服务收到“有新事件”的提醒，再主动拉取。

relay_service
  可选 deltabox 中继服务只传递事件通知或加密数据，不保存明文。
```

可见性策略：

```text
metadata_first
  其他设备先看到文件名、时间、类型、大小、缩略图状态。

preview_first
  优先同步缩略图、OCR、摘要等轻量数据。

content_on_demand
  原文件内容按需下载，避免每台设备都占用完整空间。
```

因此，手机上传大视频后，桌面端可以很快看到这个文件已存在，但原始视频可以在用户点击、后台空闲或策略要求时再下载。

离线操作：

```text
手机离线新增文件
桌面端离线重命名同一文件夹
两台设备恢复联网后交换事件
系统按事件时间、父版本和冲突策略合并
```

冲突处理原则：

- 不自动丢弃任一设备的修改
- 内容冲突保留多版本
- 命名冲突自动生成冲突副本或要求用户选择
- 标签冲突尽量合并
- 删除与修改冲突时保留可恢复 tombstone
- Agent 执行的批量操作必须有操作批次 ID，便于撤销

一致性模型：

```text
本地立即可见
远端最终一致
局域网或推送下接近实时
共享后端轮询下允许秒级到分钟级延迟
```

用户界面应显示：

- 文件是否已上传完成
- 哪些设备已看到该事件
- 哪些设备已下载原文件
- 当前是否存在同步延迟
- 冲突文件和待处理操作

## 12. 身份、账号与鉴权

deltabox 不应默认依赖中心化账号系统。账号和鉴权需要分层设计，避免把“登录 deltabox 官方账号”变成使用软件的前提。

### 12.1 身份模型

deltabox 至少需要区分四类身份：

```text
Local User Identity
  用户本地身份，代表这个 deltabox vault 的拥有者。

Device Identity
  每台设备的身份，用于多设备同步、签名、撤销。

Storage Backend Credential
  S3、OSS、邮箱、WebDAV、第三方网盘等后端的访问凭证。

Sharing Identity
  用于和其他用户共享文件或文件夹的身份。
```

可选地支持：

```text
DeltaBox Account
  官方账号或自建账号服务，用于设备发现、配置同步、付费服务、托管中继等增强能力。
```

### 12.2 本地身份

第一次创建 deltabox vault 时生成本地用户身份。

本地身份包含：

- user_id
- master public key
- encrypted master key
- recovery material
- vault_id

本地身份不要求注册官方账号。用户可以只使用本地身份和自带存储后端运行 deltabox。

### 12.3 设备鉴权

每台设备都有独立设备密钥。

设备加入流程：

```text
1. 新设备生成 device key pair
2. 老设备展示二维码或配对码
3. 新设备发起加入请求
4. 老设备确认设备名称和权限
5. 老设备加密传递必要密钥
6. 新设备获得 vault 访问权限
7. event log 记录 device.added
```

设备权限可以分级：

```text
full_access
  可读取、写入、同步、管理后端和标签。

read_only
  只能读取和搜索。

index_only
  可生成索引，但不能导出明文文件。

backup_node
  只能存储加密 chunk，不能解密内容。

limited_scope
  只能访问指定文件夹、标签或项目。
```

设备撤销：

```text
1. 用户在可信设备上撤销旧设备
2. 写入 device.revoked 事件
3. 后续同步拒绝旧设备签名
4. 高安全模式下触发密钥轮换
```

需要说明：如果旧设备已经离线持有明文或旧密钥，撤销无法让已下载内容消失，只能阻止后续访问和同步。

### 12.4 存储后端鉴权

存储后端凭证独立于 deltabox 用户身份。

示例：

```text
S3 / OSS
  access key、secret key、bucket、endpoint、region

Email
  IMAP / SMTP host、port、OAuth token 或 app password

WebDAV
  URL、username、password 或 token

Third-party Drive
  OAuth token、refresh token、scope
```

凭证保存策略：

- 优先使用系统钥匙串
- 凭证在本地加密保存
- 不把后端凭证明文写入 manifest
- 多设备同步凭证时必须经过端到端加密
- 支持每台设备使用不同后端凭证

### 12.5 可选 deltabox 账号

deltabox 可以提供官方账号或允许用户自建账号服务，但它只能是增强能力。

可选账号可用于：

- 设备发现
- 推送通知
- 订阅和付费管理
- 托管中继
- 共享邀请
- 模型服务额度
- 配置备份

不能强制依赖官方账号的能力：

- 打开本地 vault
- 访问已有文件
- 使用本地索引
- 使用自带存储后端
- 导出数据
- 恢复文件

### 12.6 Session 和解锁

客户端启动时应区分“登录”和“解锁”。

```text
解锁 vault
  使用密码、系统生物识别、硬件密钥或恢复密钥解密本地 master key。

登录 deltabox account
  访问可选的官方或自建在线服务。
```

用户不登录 deltabox account，也应该可以解锁并使用本地 vault。

### 12.7 Agent 鉴权

Agent 不继承用户的全部权限。Agent 应使用短期、受限的 capability token 调用工具。

示例：

```json
{
  "capability_id": "cap_01J...",
  "allowed_tools": ["search_files", "read_file_metadata"],
  "scope": {
    "folders": ["folder_work"],
    "mime_groups": ["document", "image"],
    "allow_plaintext": false
  },
  "expires_at": "2026-07-09T13:00:00Z"
}
```

危险操作必须单独确认：

```text
删除文件
覆盖文件
公开分享
导出明文
发送文件内容到外部模型
修改大量标签或移动大量文件
```

### 12.8 分享鉴权

共享文件或文件夹时，应使用独立共享身份和共享密钥，不直接暴露用户 master key。

共享模式：

```text
link_share
  通过加密链接分享，只读访问。

identity_share
  分享给某个已知身份，可支持后续权限调整。

folder_collaboration
  多人协作文件夹，后续阶段实现。
```

分享权限：

- 只读
- 可评论
- 可上传
- 可编辑
- 到期时间
- 下载限制
- 是否允许 Agent 读取

去中心化分享的限制必须明确：接收方已经下载的明文无法强制撤回。

### 12.9 审计日志

关键鉴权行为应进入审计日志：

- 设备加入
- 设备撤销
- 后端凭证新增或删除
- 共享链接创建
- 共享权限变更
- Agent 读取明文
- 外部模型调用
- 批量标签或文件修改

审计日志应默认本地保存，可选择端到端加密同步。

## 13. 外部智能体集成

deltabox 后续应支持被外部智能体调用，例如 Codex、Claude Code、ChatGPT、IDE Agent 或用户自建 Agent。

集成目标：

```text
让外部智能体在用户授权范围内搜索、读取、整理和操作 deltabox 文件，
但不能绕过 deltabox 的权限、加密、审计和用户确认机制。
```

### 13.1 MCP Server

MCP 是 deltabox 对外暴露能力的首选标准接口。

MCP 官方将其定义为连接 AI 应用和外部系统的开放标准，可让 AI 应用访问数据源、工具和工作流。Claude Code 文档也说明 MCP server 可以让 Claude Code 访问外部工具、数据库和 API。

deltabox 可以提供：

```text
deltabox-mcp-server
  本地 stdio MCP server，适合桌面端和本机 Agent。

deltabox-mcp-http
  远程 HTTP MCP server，适合私有服务器、NAS 或团队环境。
```

MCP tools 示例：

```text
search_files
  根据结构化条件、关键词或自然语言查询搜索文件。

read_file_metadata
  读取文件 metadata、标签、时间、大小、存储位置。

read_file_text
  在权限允许时读取 OCR、ASR、文档正文或摘要。

read_file_preview
  返回缩略图、关键帧、片段摘要或安全预览。

tag_files
  给文件增加、修改或移除标签。

move_files
  移动文件或批量整理。

create_share
  创建受限分享，需要用户确认。

summarize_files
  基于本地索引或授权内容生成摘要。
```

MCP resources 示例：

```text
deltabox://folders/{folder_id}
deltabox://files/{file_id}/metadata
deltabox://files/{file_id}/summary
deltabox://tags
deltabox://recent
deltabox://search/{query_id}
```

MCP prompts 示例：

```text
find-work-documents
organize-receipts
summarize-project-folder
prepare-file-sharing
```

### 13.2 Skills

Skill 更适合描述“怎样使用 deltabox 完成某类任务”，而不是直接承载底层文件 API。

Codex 的 Agent Skills 文档说明，skill 是可复用工作流的编写格式，可以包含 `SKILL.md`、可选脚本和参考资料，并可通过插件分发。

deltabox 可以提供官方 skills：

```text
deltabox-find-files
  指导智能体使用 deltabox MCP 搜索文件，并解释结果。

deltabox-organize-files
  指导智能体先生成整理计划，再调用工具执行，并要求用户确认危险操作。

deltabox-summarize-folder
  指导智能体读取摘要、OCR、ASR 和文档正文，生成项目资料总结。

deltabox-privacy-review
  指导智能体检查哪些文件可能被外部模型读取，并生成风险说明。
```

Skill 与 MCP 的关系：

```text
MCP
  提供可调用工具、资源和数据接口。

Skill
  提供任务流程、操作规范、权限提醒和使用示例。
```

### 13.3 权限与确认

外部智能体不能直接继承用户的全部 deltabox 权限。

每个外部 Agent 连接 deltabox 时，应获得独立授权：

```text
允许访问哪些 vault
允许访问哪些文件夹、标签或项目
是否允许读取正文
是否允许读取原文件
是否允许修改标签
是否允许移动文件
是否允许创建分享
是否允许发送内容到外部模型
授权有效期
```

危险操作必须通过 deltabox 客户端确认：

```text
删除文件
覆盖文件
公开分享
导出明文
批量移动
批量修改标签
发送文件内容到外部模型
```

### 13.4 本地优先集成

默认推荐本地 MCP server：

```text
外部 Agent -> 本地 deltabox MCP server -> 本地 vault / 本地索引 / 已配置后端
```

优点：

- 文件内容不需要经过 deltabox 官方服务器
- 可以复用本地索引和本地权限
- 用户可以随时关闭 MCP server
- 更符合去中心化和隐私优先设计

远程 MCP server 适合：

```text
NAS
家庭服务器
团队私有部署
企业内网
长期运行的自动化 Agent
```

### 13.5 审计和可撤销

所有外部智能体操作应进入审计日志：

- 哪个 Agent 调用了哪个工具
- 访问了哪些文件或 metadata
- 是否读取明文
- 是否修改标签或移动文件
- 是否创建分享
- 是否调用外部模型
- 用户是否确认

用户应能随时：

- 禁用某个 Agent
- 撤销某个 MCP token
- 关闭 MCP server
- 查看操作历史
- 回滚支持撤销的批量操作

## 14. 分享和协作

分享是去中心化系统的难点，应分阶段实现。

### 14.1 第一阶段

只支持导出文件或生成临时分享包。

### 14.2 第二阶段

支持基于共享密钥的只读分享。

### 14.3 第三阶段

支持协作文件夹、成员权限、冲突解决和撤销。

需要注意：去中心化系统中的权限撤销无法完全等同中心化云盘。接收方已经下载的明文无法强制收回。

## 15. MVP 范围

### 15.1 MVP 必须包含

- 本地目录后端
- S3 compatible 后端
- 文件分片
- chunk hash
- manifest
- SQLite metadata database
- 文件名搜索
- 基础全文搜索
- 图片 EXIF 解析
- OCR 索引
- 用户标签
- 基础标签规则
- 简单自然语言解析
- 本地 vault 解锁
- 设备身份基础模型
- 后端凭证本地加密保存
- CLI 或 Desktop 原型

### 15.2 MVP 可以暂缓

- 邮箱后端
- 复杂多设备同步
- 端到端共享
- 视频语义分析
- 视频自然语言摘要
- 人脸识别
- 人脸自动命名
- 完整 Agent 自动执行
- 多人协作
- 官方 deltabox 账号
- 复杂 RBAC 权限管理
- MCP server
- Codex / Claude Code skills

### 15.3 MVP 示例体验

用户把一批照片和文档导入 deltabox。

系统完成：

```text
扫描文件
计算 hash
生成 manifest
保存到本地后端或 S3
提取 EXIF
执行 OCR
应用用户标签和标签规则
写入 SQLite
建立全文索引
```

用户可以搜索：

```text
2025 图书馆 照片
工作规划 今年
发票 三月
```

如果接入 LLM，用户可以输入：

```text
帮我找去年我在图书馆拍的包含读书笔记的照片
```

系统返回候选文件并解释匹配原因。

## 16. 技术路线建议

### 16.1 Core

建议使用 Rust 实现核心库：

- 文件分片
- hash
- manifest
- 加密
- 存储 adapter
- 同步引擎

原因：

- 跨平台
- 性能稳定
- 适合处理文件和加密
- 可绑定到桌面端、移动端和 CLI

### 16.2 Desktop

可选：

- Tauri
- Electron

如果核心使用 Rust，Tauri 更自然。

### 16.3 Database

本地数据库：

- SQLite
- SQLite FTS5

后续可引入：

- Tantivy
- LanceDB
- sqlite-vec

### 16.4 OCR

优先级：

1. 平台原生 OCR
2. PaddleOCR
3. Tesseract
4. 用户自定义 OCR 服务

### 16.5 Agent

Agent 层应独立于具体模型服务。

不要在核心业务里写死某个 LLM API。

## 17. 同类产品与参考

目前没有一个成熟产品完整等同于 deltabox 的目标形态，但可以从多个方向参考。

### 17.1 Syncthing

官网：https://syncthing.net/

定位：

```text
去中心化、多设备、连续文件同步
```

可参考点：

- 无中心服务器的同步模型
- 多设备发现和连接
- 用户掌控数据存放位置
- 文件同步状态和冲突处理

局限：

- 更偏同步工具，不是完整网盘产品
- 不提供统一多后端对象存储抽象
- 不以 AI 索引、自然语言搜索和 Agent 为核心

### 17.2 Nextcloud

官网：https://nextcloud.com/

定位：

```text
自托管个人云盘和协作平台
```

可参考点：

- 文件浏览、同步、分享、权限
- 桌面端和移动端体验
- 标签、预览、协作功能
- 插件生态

局限：

- 典型部署仍依赖用户自己的中心服务器
- 去中心化和多后端迁移不是核心模型
- AI 能力通常作为扩展功能，而不是底层索引架构的一部分

### 17.3 Tahoe-LAFS

官网：https://tahoe-lafs.org/

定位：

```text
安全、去中心化、容错的分布式文件系统
```

可参考点：

- 加密后分布式存储
- 多服务器容错
- 存储节点不需要可信
- 数据可恢复性设计

局限：

- 更偏底层存储系统
- 普通用户产品体验较弱
- 不包含现代网盘、媒体索引和 AI Agent 体验

### 17.4 Immich

官网：https://immich.app/

文档：https://docs.immich.app/

定位：

```text
自托管照片和视频备份管理
```

可参考点：

- 照片和视频备份体验
- 人脸识别和人物命名
- 地图、时间线、相册
- 移动端照片自动备份

局限：

- 主要面向照片和视频，不是通用网盘
- 底层存储迁移和多后端副本策略不是核心
- AI Agent 和通用文档语义搜索不是核心

### 17.5 PhotoPrism

官网：https://www.photoprism.app/

定位：

```text
隐私优先的 AI 照片和视频管理应用
```

可参考点：

- 照片自动分类
- 图片搜索
- 人脸识别
- 私有部署
- 媒体库索引

局限：

- 主要聚焦媒体资产
- 不覆盖完整文件系统、版本、迁移和多后端策略
- Agent 化文件操作不是核心方向

### 17.6 Dropbox Dash

官网：https://dash.dropbox.com/

定位：

```text
面向工作资料的 AI 搜索和知识检索
```

可参考点：

- 自然语言搜索
- 跨数据源查找资料
- AI 摘要和问答体验
- 面向工作流的检索产品形态

局限：

- 中心化服务
- 用户不直接掌控底层存储、索引和模型调用边界
- 不强调去中心化和自带存储后端

### 17.7 rclone

官网：https://rclone.org/

定位：

```text
支持大量云存储后端的同步和迁移工具
```

可参考点：

- 多云存储 adapter
- 云端之间迁移
- 命令行批处理
- 存储后端兼容性

局限：

- 偏底层工具，不是终端用户网盘产品
- 不提供 AI 索引和自然语言检索
- 不管理文件语义、标签和 Agent 操作

### 17.8 deltabox 的差异化

deltabox 应组合以上产品的优点，但保持自己的核心边界：

```text
Syncthing 的去中心化同步思想
+ Nextcloud 的个人云盘体验
+ Tahoe-LAFS 的不信任存储后端思路
+ Immich / PhotoPrism 的媒体智能索引
+ Dropbox Dash 的自然语言检索体验
+ rclone 的多后端兼容能力
```

deltabox 的独特定位：

```text
用户可控存储
通用文件网盘
文件可迁移
多后端可选副本策略
本地优先智能索引
用户标签和规则
OCR / ASR / 图片 / 视频语义索引
LLM 可选增强
Agent 工具接口
```

关键区别：

- 不要求用户必须使用 deltabox 官方服务器
- 不把 LLM 作为基础功能的硬依赖
- 不把文件固定绑定到单一存储后端
- 不只服务照片，也服务通用文件和工作资料
- 不只做同步，也做可解释搜索、标签、摘要和智能体操作

## 18. 风险和挑战

### 18.1 邮箱后端限制

邮箱适合去中心化和普及性，但不适合作为唯一主存储。

### 18.2 索引体积增长

OCR、缩略图、embedding 会持续增长。必须支持索引分层、清理、压缩和重建。

### 18.3 隐私泄露

AI 能力很容易把文件内容发送到外部服务。必须设计明确的权限和确认机制。

### 18.4 多设备冲突

离线修改、重复上传、不同设备整理文件都会带来冲突。MVP 阶段应优先保留多版本，而不是自动覆盖。

### 18.5 分享撤销

去中心化分享无法完全撤销已下载明文。产品上需要清楚表达边界。

### 18.6 模型不确定性

LLM 可能误解用户意图。涉及删除、覆盖、公开分享的动作必须确认。

## 19. 里程碑

### Milestone 1: Local Prototype

- CLI
- 本地目录后端
- manifest
- SQLite 元数据
- 文件名搜索
- EXIF 提取

### Milestone 2: Search Prototype

- OCR
- PDF/docx 文本提取
- SQLite FTS5
- 用户标签
- 基础标签规则
- 搜索结果解释
- 简单自然语言解析

### Milestone 3: Storage Prototype

- S3 compatible backend
- chunk 上传下载
- 多 location 记录
- 基础恢复

### Milestone 4: AI Enhanced Prototype

- Model Provider 抽象
- LLM 查询规划
- search_files 工具
- read_file_text 工具
- 总结和候选结果解释

### Milestone 4.5: Media Intelligence Prototype

- 图片语义标签
- 人脸聚类和用户命名
- 视频关键帧抽取
- 视频关键帧 OCR
- 视频音频 ASR
- 视频结构化摘要

### Milestone 5: Desktop App

- Tauri 桌面端
- 文件浏览
- 上传下载状态
- 搜索框
- 后端配置
- 索引状态

### Milestone 6: Privacy and Sync

- 客户端加密
- 多设备加入
- event log 同步
- 冲突处理
- 敏感文件策略

## 20. 开放问题

- 文件名是否默认加密？
- manifest 是否默认全部加密，还是保留部分可检索元数据？
- 邮箱后端优先支持哪些服务商？
- 手机端是否参与完整索引，还是只做轻量索引？
- 是否提供官方中继服务，还是完全用户自带后端？
- embedding 默认使用本地模型还是显式关闭？
- 分享功能是否进入第一版产品？
- 是否需要兼容现有目录结构，还是使用专用对象布局？

## 21. 当前结论

deltabox 应设计为 AI 增强型应用，而不是 AI 强依赖应用。

核心系统应先保证：

```text
文件可靠保存
数据可迁移
后端可替换
索引可重建
隐私默认保护
工具接口清晰
```

在此基础上，LLM 负责增强体验：

```text
理解自然语言
规划查询
解释结果
总结文件
辅助整理
安全调用工具
```

这样 deltabox 既能保留去中心化软件的可靠性和自主性，也能为未来智能体能力做好架构准备。
