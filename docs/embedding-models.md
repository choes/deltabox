# 开源 Embedding 模型调研

> 调研时间：2026 年 8 月
> 背景：`design.md` 中定义的 Embedding 模型需支持**文本、图片、音频的相似搜索和语义检索**，且**默认本地生成**。本文整理当前开源可选方案，为模型选型提供参考。

## 一、文本 Embedding（语义搜索主力）

### 旗舰级（多语言，适合自建 RAG / 语义检索）

| 模型 | 提供方 | 许可 | 说明 |
|---|---|---|---|
| **Qwen3-Embedding 系列**（0.6B / 4B / 8B） | 阿里 | Apache 2.0 | 目前开源界的实际王者。支持 100+ 语言、32K 上下文、Matryoshka 可变维度。8B 质量最高，4B 通常是生产部署的性价比首选 |
| **KaLM-Embedding-Gemma3-12B** | 腾讯 | 腾讯社区许可 | MMTEB 榜单常客第一名，但许可不是 Apache，商用需评估 |
| **llama-embed-nemotron-8b** | NVIDIA | NVIDIA 许可 | 与 Qwen3-8B 相当，商用需注意许可条款 |

### 轻量级（适合本地 / 边缘设备）

| 模型 | 提供方 | 许可 | 说明 |
|---|---|---|---|
| **EmbeddingGemma-300M** | Google | Gemma 许可 | 500M 参数以下最佳，手机级设备友好 |
| **BGE-M3** | BAAI | MIT | 单个模型同时输出 dense + sparse + multi-vector，100+ 语言，8K 上下文 |
| **Qwen3-Embedding-0.6B** | 阿里 | Apache 2.0 | 高效基线 |
| **multilingual-e5-large-instruct** | Microsoft | MIT | 560M 参数的老牌强者 |
| **Granite Embedding 系列** | IBM | Apache 2.0 | 企业友好 |
| **Nomic Embed Text** | Nomic AI | Apache 2.0 | 训练数据完全开放 |

## 二、图片 / 多模态 Embedding（对应 design.md 的图片相似搜索）

| 模型 | 提供方 | 说明 |
|---|---|---|
| **SigLIP 2** | Google | CLIP 的现代替代品，开源默认选择，多语言，无许可障碍 |
| **Jina Embeddings v4** | Jina AI | 3.8B，文本+图片统一向量空间，支持 late interaction，对富视觉文档（表格、图表）很强。注意：部分权重为 CC-BY-NC（非商用） |
| **Qwen3-VL-Embedding-2B** | 阿里 | 同时处理文本/图片/**视频**，跨模态检索基准上超过闭源 API |
| **JinaCLIP v2** | Jina AI | 现代化 CLIP，Matryoshka 压缩可到 64 维 |
| **ColPali / ColQwen2** | 社区 | 视觉文档检索（PDF/扫描件 RAG）的首选，多向量方案 |
| **DINOv2/v3** | Meta | 纯视觉相似度（以图搜图）优于 CLIP，但不支持文本跨模态 |

## 三、音频 Embedding

开源选择相对较少：

- **CLAP**（Contrastive Language-Audio Pretraining）— 音频-文本跨模态的经典开源方案
- **ASR → 文本 embedding 间接路线** — 用 Whisper 等 ASR 模型转写后再做文本 embedding，与 `design.md` 中 ASR 模块的设计正好吻合，是当前更实用的路线

## 四、结合 deltabox 的选型建议

针对设计文档「**embedding 默认本地生成**」的策略：

| 用途 | 推荐 |
|---|---|
| 文本语义搜索 | Qwen3-Embedding-0.6B / 4B（本地） |
| 图片相似 / 以图搜图 | SigLIP 2 或 Qwen3-VL-Embedding-2B |
| 中文场景 | Qwen3 系列、BGE-M3 都有很好支持 |
| 音频 | Whisper ASR + 文本 embedding |

### 许可注意事项

- **优先选**（商用无障碍）：Apache 2.0（Qwen3、Granite、Nomic）、MIT（BGE-M3、multilingual-e5）
- **需评估**：Jina 部分权重为 CC-BY-NC-4.0（仅限非商用）、KaLM 为腾讯社区许可、NVIDIA 模型有其自有许可
- **隐私契合度**：Qwen3-Embedding-0.6B 和 EmbeddingGemma-300M 体积小，适合在客户端设备上本地运行，符合「索引默认本地保存、embedding 默认本地生成」的安全策略

## 五、待决问题（对应 design.md §12）

`design.md:2380` 留下的开放问题「embedding 默认使用本地模型还是显式关闭？」——结合本次调研，建议：

- **默认本地模型**：Qwen3-Embedding-0.6B（约 1.2GB，Apache 2.0，中英文俱佳）
- 用户可在设置中切换更大模型（4B）或显式关闭
- 敏感文件模式按 `design.md:1354` 要求支持「不生成 embedding」

## 参考资料

- [Top 10 Open/Closed-Source Embedding Models 2026 (explainx.ai)](https://explainx.ai/blog/top-10-open-closed-source-embedding-models-2026)
- [MMTEB Benchmark Paper (arXiv)](https://www.arxiv.org/pdf/2502.13595)
- [Granite Embedding Multilingual R2 (Hugging Face / IBM)](https://huggingface.co/blog/ibm-granite/granite-embedding-multilingual-r2)
- [Best Multimodal Embedding Models in 2026 (Mixpeek)](https://mixpeek.com/curated-lists/best-multimodal-embedding-models)
- [CLIP vs SigLIP vs Jina CLIP (dreaming.press)](https://dreaming.press/posts/2026-06-22-clip-vs-siglip-vs-jina-clip-multimodal-embeddings.html)
- [SigLIP 2 (arXiv)](https://arxiv.org/html/2502.14786)
- [Multimodal Embeddings on GPU Cloud (Spheron)](https://www.spheron.network/blog/multimodal-embedding-models-gpu-cloud-siglip2-jinaclip-cohere/)
- [jina-embeddings-v4 (arXiv)](https://arxiv.org/pdf/2506.18902v3)
- [How to Choose the Best Embedding Model for RAG in 2026 (Milvus)](https://milvus.io/ja/blog/choose-embedding-model-rag-2026.md)
- [MTEB Leaderboard (Hugging Face)](https://huggingface.co/spaces/mteb/leaderboard)
