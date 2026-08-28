# Rust 概念学习指南 —— 以 deltabox 代码为教材

面向有 C++ 背景的读者。目标不是"会写 Rust"，而是"**能读懂、能审查** deltabox 这类 Rust 代码"。
每个阶段只讲**概念和心智模型**，语法细节请随用随查，不必预先记忆。

## 怎么用这份指南

每个阶段的流程：

1. **读概念**：理解这一阶段的"为什么"（都有 C++ 对照）
2. **看证据**：打开指南指出的 deltabox 源码位置，找到概念的实际体现
3. **做自测**：能回答自测问题才算过关，答不上就回到第 1 步
4. **卡住就问 AI**：把代码片段贴给 AI 问"这里为什么这样写"，比啃书快得多

阶段按依赖关系排序，建议按顺序推进。每个阶段约 1~3 小时。

---

## 阶段 0：心智模型 —— 编译器是你的结对程序员

**核心概念**：C++ 里"资源管理正确性"靠程序员自律（RAII 惯例、const 纪律、不要悬垂指针）；Rust 把这套纪律变成了**编译期强制规则**。编译器报错不是刁难你，而是在替你 review。读 Rust 代码时，很多"奇怪写法"（到处都是 `.clone()`、参数用 `&str` 而不是 `String`）都是在配合这套规则。

**C++ 对照**：相当于把 code review 清单里的"谁拥有这块内存？生命周期够不够长？有没有数据竞争？"全部交给编译器检查。

**看证据**：不需要看代码，先建立预期——后面每个阶段都是这条主线的展开。

**自测**
- 为什么 Rust 代码里很少需要写析构函数？
- 如果编译器保证了内存安全，review Rust 代码时你的注意力应该放在哪？（提示：业务逻辑、错误处理路径、unwrap/panic 点）

---

## 阶段 1：所有权 —— 每个值有且只有一个主人

**核心概念**：值默认**移动**（move）而不是拷贝。把一个值传给函数，所有权就移交了，原来的变量作废。想保留就用 `.clone()` 显式拷贝。没有 GC，也没有共享所有权的默认假设。

**C++ 对照**：`std::unique_ptr` 的移动语义被推广到**所有类型**，且移动后使用旧变量是编译错误而非"空指针"。`.clone()` ≈ 显式调用拷贝构造。

**看证据**
- `crates/deltabox-core/src/backends.rs:24` 构造 `StorageBackendRecord`：`now.clone()` 给了 `created_at`，`now` 本体给了 `updated_at`——最后一个使用者可以直接拿走所有权，不用克隆
- `crates/deltabox-core/src/lib.rs:27` `Vault` 结构体持有 5 个 `PathBuf`——它是这些路径的唯一主人，析构时自动释放

**自测**
- 为什么 `backends.rs:29` 是 `now.clone()` 而第 30 行直接写 `now`？反过来写行不行？
- 看到 `.clone()` / `.to_owned()` 时应该想什么？（这是一次显式拷贝，值得问"必要吗"）

---

## 阶段 2：借用 —— 临时访问权与读写互斥

**核心概念**：不想移交所有权，就**借用**：`&T`（只读引用，可多人同时持有）或 `&mut T`（可变引用，**全局独占**）。规则一句话：**可读可写不能同时存在**。这条规则在编译期消灭了整个"迭代器失效 / 数据竞争"的 bug 类别。

**C++ 对照**：`const T&` 与 `T&`，但编译器强制"`T&` 存在期间没有任何其他引用"。

**看证据**
- `crates/deltabox-core/src/backends.rs:12` 起，整个 `impl Vault` 块所有方法都是 `&self`——Vault 的方法只读取/操作内部资源，从不消费 Vault 本身
- `crates/deltabox-core/src/storage/mod.rs:8` `fn put_chunk(&self, chunk_id: &str, data: &[u8])`——参数全是借用：函数用完即还，调用者保留所有权

**自测**
- 为什么 `&self` 方法意味着"调用后 Vault 还能继续用"？如果写成 `self`（按值）会发生什么？
- 借用的生命周期由谁保证？（编译器的借用检查器，所以你几乎看不到悬垂引用）

---

## 阶段 3：拥有型 vs 视图型 —— 成对出现的类型

**核心概念**：Rust 标准库里数据类型成对出现：`String`（拥有）/ `&str`（视图）、`PathBuf` / `&Path`、`Vec<u8>` / `&[u8]`。**API 参数用视图型，存储用拥有型**——这样函数不关心调用者的数据从哪来（栈上字面量、堆上 String 都行）。

**C++ 对照**：`std::string` / `std::string_view`，但 `&str` 额外保证 UTF-8 合法，且视图绝不比本体活得久（编译器强制）。

**看证据**
- `crates/deltabox-core/src/backends.rs:130` `local_backend_by_id(&self, backend_id: &str)`——参数是视图，内部需要存起来时才 `.to_owned()` 变成 `String`（如 `backends.rs:25`）
- `crates/deltabox-core/src/manifest.rs` 里 `FileStatus` 等类型的定义，对比 `lib.rs:42` `FileRecord` 全用拥有型 `String`——record 要长期存活、跨层传递，必须拥有自己的数据

**自测**
- 函数签名写 `name: &str` 和 `name: String`，对调用者分别意味着什么？哪个更通用？
- 为什么 `FileRecord` 的字段不能用 `&str`？（提示：record 从数据库读出来要返回给调用者，借谁的呢？）

---

## 阶段 4：错误即值 —— Result、Option 与 `?`

**核心概念**：没有异常。可能失败 → 返回 `Result<T, E>`；可能不存在 → 返回 `Option<T>`。错误是**普通返回值**，调用链上每个环节都必须显式处理：`?` 运算符 = "出错就提前返回，把错误往上传"。这套机制让错误路径在代码里**可见、可数、可审查**。

**C++ 对照**：`std::expected<T, E>` + `std::optional<T>`；`?` ≈ 每个调用点自动 `if (!ok) return err;`。

**看证据**
- `crates/deltabox-core/src/backends.rs:134-136`：`find_backend(...)?` 拿到 `Option`，`.ok_or_else(|| anyhow!(...))` 把"没找到"转成错误——这是 Option → Result 的标准桥接
- `crates/deltabox-core/src/backends.rs:243` `json_string` 函数：`get → and_then → map → ok_or_else` 的完整组合子链，没有一行 if
- `crates/deltabox-server/src/handlers.rs:23` `spawn_blocking(f).await??` —— **两个 `?`**：第一个解 tokio 的任务错误，第二个解业务错误，两层错误类型不同但都能传播

**自测**
- `unwrap()` 和 `?` 的区别是什么？review 时看到 `unwrap()` 应该问什么？（这个 panic 能被用户输入触发吗？）
- `anyhow` 的作用是什么？（把各种具体错误类型统一成一个可携带上下文的黑盒错误，应用层用；库才需要细分错误类型）

---

## 阶段 5：迭代器与闭包 —— 声明式的数据管道

**核心概念**：迭代器是**惰性**的：`map`/`filter` 只是描述管道，`collect()` 才真正执行。闭包（`|x| ...`）可以捕获环境变量，`move ||` 表示把捕获的变量**所有权搬进闭包**。这是一种"描述要做什么，而不是怎么循环"的风格。

**C++ 对照**：C++20 ranges 的 view 管道 + lambda 捕获；`move ||` ≈ lambda 按值捕获，但语义是移动。

**看证据**
- `crates/deltabox-core/src/backends.rs:66-68`：`query_map` 产生"每行可能失败"的迭代器，`collect::<rusqlite::Result<Vec<_>>>()` 把它们聚合——任何一行失败，整体就是那个错误。这是 Rust 迭代器最精妙的模式之一
- `crates/deltabox-server/src/handlers.rs:29` `run(move || state.vault.list_files(false))`——`move` 把 `state` 搬进闭包，因为闭包要在**另一个线程**上跑，必须拥有自己的数据

**自测**
- `map` 之后不加 `collect` 会发生什么？（什么都不发生——惰性）
- 为什么 `handlers.rs` 里几乎每个闭包都带 `move`？（要跨线程，借用在另一个线程里无法保证有效）

---

## 阶段 6：模式匹配与穷尽性 —— enum 是带数据的标签联合

**核心概念**：`match` 不只是 switch：它**解构**值、绑定内部数据，并且编译器强制**穷尽所有情况**（少一个分支就报错）。enum 的每个变体可以携带不同类型的数据，是"状态机"的一等表达方式。

**C++ 对照**：`std::variant` + `std::visit`，但语法原生、且编译器保证不会漏掉某个 variant。

**看证据**
- `crates/deltabox-core/src/backends.rs:155` `match record.backend_type.as_str()`——按后端类型分派到 local / s3 / 其他，第三支 `other =>` 兜住所有未知类型
- `crates/deltabox-core/src/backends.rs:219` `Some(prefix) if !prefix.is_empty()`——带**守卫条件**的分支：既是 `Some` 又非空才走这条路
- `crates/deltabox-server/src/handlers.rs:39` `while let Some(field) = ...`——循环消费"可能耗尽"的流
- `crates/deltabox-core/src/manifest.rs` 看 `FileStatus` 的定义：文件状态用 enum 表达，非法状态在类型层面就不存在

**自测**
- `match` 少了 `_ =>` 兜底分支，什么时候能编译过？（分支已经穷尽时——编译器知道）
- 相比 `if status == "active"` 这种字符串比较，enum 状态机在重构时有什么优势？（改名/加状态时编译器指出所有需要改的地方）

---

## 阶段 7：trait —— 接口、组合与两种多态

**核心概念**：trait ≈ 接口/抽象基类，但**没有继承体系**，类型与接口解耦（可为任何类型事后实现 trait）。两种多态：
- **静态分发** `impl Trait` / 泛型：编译期为每个具体类型生成代码，零开销 ≈ C++ 模板
- **动态分发** `Box<dyn Trait>`：运行期虚表 ≈ `unique_ptr<Base>`，用于"运行期才知道具体类型"的场景

**C++ 对照**：concepts + 虚基类的合体，但组合关系是平面的，没有菱形继承问题。

**看证据**
- `crates/deltabox-core/src/storage/mod.rs:6` `StorageBackend` trait：5 个方法定义了存储后端契约
- `crates/deltabox-core/src/backends.rs:148` 返回 `Box<dyn StorageBackend>`——后端类型从数据库字符串（`"local"`/`"s3"`）决定，运行期才知道，所以必须动态分发
- `crates/deltabox-core/src/extractors.rs:46` `Vec<Box<dyn TextExtractor>>`——多态注册表：一批不同提取器放进同一个容器
- `crates/deltabox-core/src/storage/local.rs` 和 `s3.rs`：看同一个 trait 的两种实现风格

**自测**
- 为什么 `storage_backend_by_id` 不能用泛型返回具体类型？（返回类型必须在编译期确定，而后端类型存在数据库里）
- `Box<dyn Trait>` 里的 `Box` 是干什么的？（trait 对象大小不定，必须放在堆上通过指针间接持有）

---

## 阶段 8：用类型系统建模 —— 让非法状态无法表示

**核心概念**：Rust 社区的设计哲学：把业务约束编码进类型，让"写错"变成编译错误。手段包括：enum 状态机（阶段 6）、newtype 包装、`Option` 字段表达可空、以及 `derive` 宏自动生成序列化等样板。

**C++ 对照**：强类型化的极致版——C++ 靠 `explicit` 构造函数和 `[[nodiscard]]` 部分实现，Rust 把它变成主流风格。

**看证据**
- `crates/deltabox-core/src/lib.rs:36` `AddOptions { source: PathBuf, logical_path: Option<String> }`——`Option` 直接告诉读者"逻辑路径可以不提供"
- `crates/deltabox-core/src/lib.rs:41-129` 一整排 `#[derive(Debug, Clone, serde::Serialize)]` record 结构体：derive 宏 = 编译期代码生成，JSON 序列化能力零手写
- `crates/deltabox-server/src/handlers.rs:133` `#[derive(Deserialize)] struct SearchParams`——HTTP query 反序列化成强类型结构体，缺字段/类型错在框架层就被拒绝，handler 里不用再校验

**自测**
- `lib.rs` 里 `status: String`（如 `IndexJobRecord.status`）和 `manifest.rs` 里 `FileStatus` enum，哪种建模更好？为什么项目里两种都存在？（数据库边界上常退化为字符串，内存里用 enum）
- derive 出来的 `Serialize` 让 `serde_json::to_value(files)`（`handlers.rs:30`）一行完成 JSON 转换——如果 AI 给某个结构体漏加了 derive，会在哪一层报错？

---

## 阶段 9：模块系统与可见性 —— 代码的所有权边界

**核心概念**：crate（编译单元）→ module（文件/目录）→ item（函数/类型），可见性默认**私有**，`pub` 逐层放开。`pub(crate)` = 只对当前 crate 可见。这是"信息隐藏"的语言级实现，比 C++ 的头文件/include 守卫清晰得多。

**C++ 对照**：命名空间 + `static` 内部链接 + friend 的替代方案；Cargo workspace ≈ 一个 solution 里多个 project。

**看证据**
- `crates/deltabox-core/src/lib.rs:1-15`：`pub mod manifest`（对外公开）vs `mod backends`（crate 内部私有）——core 的公共 API 面从这几行就能看出来
- `crates/deltabox-core/src/backends.rs:194` `pub(crate) fn ensure_default_local_backend`——内部辅助函数，server/cli 看不见
- 注意 `Vault` 的方法分散在 `vault.rs`、`backends.rs`、`files.rs`、`tags.rs` 等多个文件的 `impl Vault` 块里——Rust 允许**同一类型的 impl 块拆开**，按功能组织而不是挤在一个类里

**自测**
- server crate 能不能直接调用 `deltabox_core` 里 `mod backends` 的私有函数？（不能——但 `Vault` 上 `pub` 的方法可以，哪怕方法定义在私有模块里；可见性看 item 本身的 `pub` 与路径可达性）
- 根 `Cargo.toml` 的 `[workspace.dependencies]` 解决了什么问题？（多个 crate 共享依赖版本，一处升级）

---

## 阶段 10：并发世界观 —— 同步核心 + 异步外壳

**核心概念**：deltabox 展示了一个典型的 Rust 架构决策：**core 全同步**（rusqlite + std::fs，简单可靠），**server 是异步壳**。两个世界之间有两座桥：
- 异步调同步：`tokio::task::spawn_blocking`——把阻塞代码丢到专用线程池，不卡住 async 运行时
- 同步调异步：`Runtime::block_on`——在同步代码里临时驱动一个 Future 到完成

闭包上 `Send + 'static` 的约束含义：**要跨线程的闭包，必须能安全地发送到另一线程（Send），且不持有任何借来的短期引用（'static）**。

**C++ 对照**：`std::async` / 线程池投递，但"能不能跨线程"由编译器按类型检查，不是靠约定。

**看证据**
- `crates/deltabox-server/src/handlers.rs:15-24` `run` 函数——整个 server 的核心模式，注释（`handlers.rs:15-17`）自己解释了为什么：core 是全同步的，每个 vault 操作都必须走 `spawn_blocking`
- `crates/deltabox-core/src/storage/s3.rs:88` `runtime.block_on(async { ... })`——反方向的桥：同步 trait 方法里调异步的 S3 SDK
- `crates/deltabox-core/src/storage/s3.rs:26` `Arc<dyn ObjectStore>`——`Arc`（原子引用计数 ≈ `shared_ptr`）只在真正需要共享所有权时出现，全项目仅此一处，说明作者默认避免共享

**自测**
- 如果在 axum handler 里直接调 `state.vault.list_files(false)`（不走 `spawn_blocking`），会发生什么？（阻塞 async 运行时线程，所有并发请求被拖慢——编译器**不会**拦你，这是 review 要点）
- `Send` 约束为什么能防止数据竞争？（不满足 Send 的类型——比如持有裸指针或 `Rc`——编译器拒绝让它跨线程）

---

## 进阶（可选，项目里用得少）

- **生命周期标注** `'a`：本项目的代码几乎不用显式标注（编译器自动推导）。知道"`&'a T` 表示这个引用至少活到 'a"即可，遇到再查
- **async 深入**（Future/pin/waker）：只有要改 server 框架层才需要；读业务代码不需要
- **宏编程**：只用现成宏（`params!`、`json!`、`anyhow!`），不需要会写

## 推荐材料（按阶段查阅，不必通读）

| 阶段 | The Rust Book 章节 |
|---|---|
| 1–3 | 第 4 章（所有权）、第 15 章（智能指针，只看 Box/Arc 两节） |
| 4 | 第 9 章（错误处理） |
| 5 | 第 13 章（闭包与迭代器） |
| 6 | 第 6 章（enum 与 match）、第 18 章（模式） |
| 7 | 第 10 章（trait）、第 17 章（trait 对象） |
| 8 | 第 19 章（newtype 一节） |
| 9 | 第 7 章（包与模块） |
| 10 | 第 16 章（并发）+ [Tokio tutorial](https://tokio.rs/tokio/tutorial) 前两节 |

速查工具：[Rust By Example](https://doc.rust-lang.org/rust-by-example/)（按场景查，不当教程读）。
