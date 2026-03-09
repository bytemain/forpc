# Cap'n Proto "Time Travel" RPC 与 forpc 实现方案

> **日期**: 2026-03-09（更新）
> **范围**: 研究 Cap'n Proto 的"时间旅行"（Promise Pipelining）机制，分析其核心原理，并提出在 forpc 中实现类似能力的方案。
> **结论**: 完整 Pipelining 对 forpc 来说过重。轻量可借鉴的模式见 [PIPELINING_LITE.md](./PIPELINING_LITE.md)。

---

## 目录

1. [什么是"时间旅行" RPC](#1-什么是时间旅行-rpc)
2. [Cap'n Proto 的实现原理](#2-capn-proto-的实现原理)
3. [forpc 当前的差距](#3-forpc-当前的差距)
4. [forpc 实现方案](#4-forpc-实现方案)
5. [分阶段实施路线图](#5-分阶段实施路线图)
6. [参考资料](#6-参考资料)

---

## 1. 什么是"时间旅行" RPC

### 1.1 问题：链式调用的延迟放大

在传统 RPC 中，如果你需要在一次调用的结果上再发起下一次调用，必须等第一次调用完成后才能开始第二次。每次调用都需要一个完整的网络往返（round-trip）：

```
传统 RPC —— 3 次链式调用需要 3 个 RTT

Client                                Server
  │                                      │
  │──── openDir("/home") ──────────────► │   RTT 1
  │◄─── dirHandle ───────────────────── │
  │                                      │
  │──── openFile(dirHandle, "a.txt") ──► │   RTT 2
  │◄─── fileHandle ──────────────────── │
  │                                      │
  │──── read(fileHandle, 0, 1024) ─────► │   RTT 3
  │◄─── data ────────────────────────── │
  │                                      │
  总延迟 = 3 × RTT
```

在高延迟场景（卫星链路、跨区域）中，延迟可能高达数百毫秒甚至数秒。如果链式调用层数更多，性能将急剧恶化。

### 1.2 解决方案：Promise Pipelining（"时间旅行"）

Cap'n Proto 的 "time travel" 核心思想：**客户端不等第一次调用完成，而是直接对"尚未返回的结果"发起后续调用**。框架将所有链式调用打包在一起发送，服务端依次执行后一次性返回最终结果：

```
Promise Pipelining —— 3 次链式调用仅需 1 个 RTT

Client                                Server
  │                                      │
  │──── openDir("/home") ──────────────► │
  │──── openFile(Q1.result, "a.txt") ──► │   全部一起发送！
  │──── read(Q2.result, 0, 1024) ──────► │
  │                                      │
  │                                      │   依次执行：
  │                                      │   1. dir = openDir("/home")
  │                                      │   2. file = openFile(dir, "a.txt")
  │                                      │   3. data = read(file, 0, 1024)
  │                                      │
  │◄─── data ────────────────────────── │   仅返回最终结果
  │                                      │
  总延迟 = 1 × RTT
```

从客户端视角看，代码写起来像是"穿越了时间"——在结果还没到达之前就已经在使用它了。

### 1.3 为什么不手写批量 API？

一种朴素的替代方案是设计批量 API（例如 `readFileFromPath("/home/a.txt")`），把多步操作合并为一次调用。但这有明显缺点：

| 手写批量 API | Promise Pipelining |
|-------------|-------------------|
| 每种组合都需要新 API | 通用机制，自动优化任意组合 |
| API 爆炸式增长 | API 保持模块化 |
| 跨服务组合困难 | 跨服务的链式调用也能优化 |
| 修改逻辑需改 API | 只需改调用链 |

---

## 2. Cap'n Proto 的实现原理

### 2.1 四张核心状态表

Cap'n Proto RPC 在每个连接上维护四张表，用于追踪调用和 Capability（能力引用）的生命周期：

```
┌─────────────────────────────────────────────────────────────────┐
│                    Cap'n Proto RPC 连接状态                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Questions Table (问题表)                                        │
│  ┌──────────────────────────────────────────────┐               │
│  │ Q1: openDir("/home")          → pending      │               │
│  │ Q2: openFile(Q1.result, ...) → pending       │  ← 引用 Q1   │
│  │ Q3: read(Q2.result, ...)     → pending       │  ← 引用 Q2   │
│  └──────────────────────────────────────────────┘               │
│                                                                 │
│  Answers Table (回答表)                                          │
│  ┌──────────────────────────────────────────────┐               │
│  │ A1: openDir result           → resolved      │               │
│  │ A2: openFile result          → resolved      │               │
│  └──────────────────────────────────────────────┘               │
│                                                                 │
│  Exports Table (导出表)                                          │
│  ┌──────────────────────────────────────────────┐               │
│  │ E1: local DirectoryImpl      → refcount: 1   │               │
│  │ E2: local FileImpl           → refcount: 1   │               │
│  └──────────────────────────────────────────────┘               │
│                                                                 │
│  Imports Table (导入表)                                          │
│  ┌──────────────────────────────────────────────┐               │
│  │ I1: remote Directory proxy   → alive         │               │
│  │ I2: remote File proxy        → alive         │               │
│  └──────────────────────────────────────────────┘               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

| 表 | 作用 | 关键字段 |
|----|------|---------|
| **Questions** | 追踪本端发出、等待对端响应的调用 | question_id, target, method, params |
| **Answers** | 追踪对端发来的调用及其响应状态 | answer_id, result, finished |
| **Exports** | 本端共享给对端的对象/Capability | export_id, object_ref, refcount |
| **Imports** | 从对端接收的 Capability 代理 | import_id, proxy, refcount |

### 2.2 Promise 引用机制

Pipeline 的核心在于：**调用的 target 可以是一个尚未解析的 Promise**。

```
消息格式（简化）：

Call {
    question_id: 3,
    target: PromisedAnswer {
        question_id: 2,        // 引用问题 2 的结果
        transform: [            // 从结果中提取特定字段
            GetPointerField(0)  // 取返回值的第 0 个字段
        ]
    },
    method: "read",
    params: { start: 0, amount: 1024 }
}
```

关键概念：

- **PromisedAnswer**：一个指向"某个 Question 的未来结果"的引用
- **Transform**：从 Promise 结果中导航到特定子字段/Capability
- 服务端在收到此消息时，会在 Q2 完成后将 Q2 的结果传给 Q3

### 2.3 Capability 传递

Cap'n Proto 的 Capability 模型受 E 语言的 CapTP 协议启发：

- **Capability** = 对象引用 + 权限（不可伪造的令牌）
- Capability 可以作为调用参数或返回值在 Peer 之间传递
- 传递时自动记录到 Export/Import 表
- 当引用计数归零时，发送 `Release` 消息释放远端资源

```
Capability 传递示例：

Peer A                                      Peer B
  │                                            │
  │── Call: getFS() ─────────────────────────► │
  │◄── Return: { fs: CapabilityRef(E1) } ──── │  B 导出 fs 对象
  │                                            │
  │   A 现在持有 I1 (import of E1)              │
  │                                            │
  │── Call: I1.open("a.txt") ────────────────► │  A 用导入的引用调用
  │◄── Return: { file: CapabilityRef(E2) } ── │  B 导出 file 对象
  │                                            │
```

### 2.4 协议级别（Level 0–4）

Cap'n Proto 定义了 5 个渐进式协议级别：

| Level | 名称 | 能力 |
|-------|------|-----|
| **0** | Bootstrap | 仅支持获取根 Capability，无对象引用传递 |
| **1** | Basic Pipelining | 支持 Promise Pipelining + Capability 引用传递 |
| **2** | Persistent Capabilities | 支持 SturdyRef —— 可持久化的 Capability 引用 |
| **3** | Three-Party Handoff | 支持三方介绍 —— A 将 B 的 Capability 介绍给 C |
| **4** | Join | 支持分布式 Capability 等价性检查 |

大多数实现停留在 Level 1，这已经覆盖了 Promise Pipelining 的核心价值。

---

## 3. forpc 当前的差距

将 forpc 的现有架构与 Cap'n Proto 的 Pipelining 机制对比：

### 3.1 现有架构概览

```
forpc 当前的调用生命周期：

1. alloc_stream_id() → 分配唯一 stream ID（奇数/偶数分区）
2. insert_pending_call() → 在 pending_calls HashMap 中注册
3. 发送 HEADERS(Call{method, metadata})
4. 发送 DATA(payload)
5. 发送 TRAILERS(Status::ok())
6. 等待 → oneshot::Receiver (unary) / mpsc::Receiver (stream)
7. 收到响应 TRAILERS → 从 pending_calls 中移除，resolve 结果
```

### 3.2 缺失的关键能力

| 能力 | Cap'n Proto | forpc 现状 | 差距 |
|------|-------------|-----------|------|
| Promise 引用 | PromisedAnswer 可引用未完成调用 | ❌ 调用结果只能本地消费 | 无法将 "未来结果" 作为参数 |
| Capability 传递 | 对象引用可在 Peer 间传递 | ❌ 只传递原始字节 | 无远程对象引用 |
| 链式调用优化 | 多个调用一次 RTT | ❌ 每次调用独立往返 | 链式调用延迟线性增长 |
| 引用追踪 | Export/Import 表 + 引用计数 | ❌ 无 | 无法管理远端对象生命周期 |
| Transform 导航 | 从结果中提取子字段 | ❌ 无 | 无法定位嵌套 Capability |

### 3.3 forpc 的优势基础

forpc 已有一些对实现 Pipelining 有利的基础：

- ✅ **对等架构**：双方都可发起调用，天然适合 Capability 双向传递
- ✅ **stream_id 多路复用**：已有 ID 分配和 pending_calls 追踪机制
- ✅ **Metadata**：可通过 metadata 传递 Promise 引用信息
- ✅ **Protobuf**：结构化消息可方便扩展新字段
- ✅ **RST_STREAM 取消**（PR #40）：已有调用取消基础设施，可为 Pipeline 取消链复用
- ✅ **超时执行**（PR #35）：`:timeout` metadata 已在三端执行

---

## 4. forpc 实现方案

### 4.1 设计原则

1. **向后兼容**：现有 Unary / Streaming 调用保持不变
2. **渐进式**：先实现 Level 0（基础），再扩展到 Level 1（Pipelining）
3. **简洁优先**：不追求 Cap'n Proto 的完整 Capability 模型，专注 Promise Pipelining 的核心价值
4. **协议扩展**：通过 metadata 和新帧类型扩展，不破坏现有帧格式

### 4.2 协议扩展

#### 4.2.1 新增 Metadata 保留键

```protobuf
// 在 Call.metadata 中新增以下保留键：
// ":pipeline-target"   — 标记此调用的 target 是一个 Promise
// ":pipeline-qid"      — 引用的 Question ID（stream_id）
// ":pipeline-field"    — 从结果中提取的字段路径（可选）
// ":cap-id"            — Capability 引用 ID
```

#### 4.2.2 扩展 Call 结构（方案 A：Metadata-Based，推荐）

利用现有 metadata 字段，不需要修改 proto 定义：

```rust
// Rust 示例：创建一个引用 Q1 结果的 Pipeline 调用
let call = Call::new("File/Read")
    .with_metadata(":pipeline-target", "true")
    .with_metadata(":pipeline-qid", "1")       // 引用 stream_id=1 的结果
    .with_metadata(":pipeline-field", "file")   // 从结果中取 "file" 字段
    .with_metadata(":timeout", "5000");
```

#### 4.2.3 扩展 Call 结构（方案 B：Proto Extension，备选）

如果需要更严格的类型检查，可以扩展 proto 定义：

```protobuf
message Call {
    string method = 1;
    map<string, string> metadata = 2;

    // Pipeline 扩展
    PipelineTarget pipeline_target = 3;
}

message PipelineTarget {
    uint32 question_id = 1;       // 引用的 Question stream_id
    repeated string field_path = 2; // 从结果中导航的字段路径
}
```

### 4.3 核心数据结构变更

#### 4.3.1 Rust 端

```rust
// ============= 新增：Pipeline 状态追踪 =============

/// 一个 Pipeline 调用的 target 描述
#[derive(Debug, Clone)]
pub struct PipelineTarget {
    /// 引用的 Question 的 stream_id
    pub question_id: u32,
    /// 从 Question 结果中提取 Capability 的字段路径
    pub field_path: Vec<String>,
}

/// 扩展 PendingCall，增加 Pipeline 等待队列
pub struct PendingCall {
    // ...现有字段...
    tx: Option<oneshot::Sender<RpcResult<Bytes>>>,
    stream_tx: Option<mpsc::Sender<Packet>>,
    unary_buffer: Option<Bytes>,

    // 新增：等待此调用完成后才能分发的 Pipeline 调用
    pipeline_waiters: Vec<PipelinedRequest>,
}

/// 一个被挂起等待依赖解析的 Pipeline 请求
#[derive(Debug)]
pub struct PipelinedRequest {
    pub stream_id: u32,           // 此 Pipeline 调用自己的 stream_id
    pub call: Call,               // 调用信息
    pub payload: Bytes,           // 请求 payload
    pub field_path: Vec<String>,  // 从依赖结果中取哪个字段
}
```

#### 4.3.2 新增：Capability 引用表（Level 1 进阶）

```rust
/// Capability 管理器
pub struct CapabilityManager {
    /// 导出表：本端对象 → export_id
    exports: HashMap<u64, Arc<dyn CapabilityHandler>>,
    next_export_id: AtomicU64,

    /// 导入表：远端 Capability → import_id
    imports: HashMap<u64, CapabilityProxy>,
}

/// Capability Handler trait
pub trait CapabilityHandler: Send + Sync {
    fn handle(&self, method: &str, payload: Bytes) -> BoxFuture<RpcResult<Bytes>>;
}

/// 远端 Capability 的本地代理
pub struct CapabilityProxy {
    peer: Arc<RpcPeer>,
    remote_id: u64,
}
```

### 4.4 请求处理流程变更

#### 4.4.1 发送端：Pipeline 调用打包

```rust
// ============= Rust 伪代码 =============

impl RpcPeer {
    /// 创建一个 Pipeline 调用（引用另一个未完成调用的结果）
    pub async fn call_on_promise(
        &self,
        promise_stream_id: u32,  // 依赖的调用的 stream_id
        field_path: Vec<String>, // 从结果中取哪个字段
        method: &str,
        payload: Bytes,
    ) -> RpcResult<Bytes> {
        let stream_id = self.alloc_stream_id();

        let call = Call::new(method)
            .with_metadata(":pipeline-target", "true")
            .with_metadata(":pipeline-qid", &promise_stream_id.to_string())
            .with_metadata(":pipeline-field", &field_path.join("."));

        // 注册 pending call
        let (tx, rx) = oneshot::channel();
        self.insert_pending_call(stream_id, PendingCall::new_unary(tx));

        // 发送 HEADERS + DATA + TRAILERS（和普通调用一样）
        self.send_headers(stream_id, &call).await?;
        self.send_data(stream_id, payload).await?;
        self.send_trailers(stream_id, Status::ok()).await?;

        // 等待响应
        rx.await.map_err(|_| RpcError::cancelled("pipeline call cancelled"))?
    }
}
```

#### 4.4.2 接收端：Pipeline 调用分发

```rust
// ============= 接收端处理逻辑（伪代码） =============

async fn handle_inbound_headers(&self, stream_id: u32, call: Call) {
    // 检查是否是 Pipeline 调用
    if call.metadata.get(":pipeline-target").map(|v| v == "true").unwrap_or(false) {
        let qid: u32 = call.metadata.get(":pipeline-qid")
            .and_then(|v| v.parse().ok())
            .expect("pipeline-qid required");
        let field_path: Vec<String> = call.metadata.get(":pipeline-field")
            .map(|v| v.split('.').map(String::from).collect())
            .unwrap_or_default();

        // 检查依赖的调用是否已经完成
        if let Some(resolved_result) = self.get_resolved_answer(qid) {
            // 依赖已解析，立即分发
            let capability = resolved_result.extract_field(&field_path);
            capability.handle(call.method, payload).await;
        } else {
            // 依赖未完成，加入等待队列
            self.add_pipeline_waiter(qid, PipelinedRequest {
                stream_id,
                call,
                payload: Bytes::new(), // DATA 帧稍后到达
                field_path,
            });
        }
    } else {
        // 普通调用，走现有路径
        self.dispatch_normal_call(stream_id, call).await;
    }
}

/// 当一个调用完成时，检查并分发等待它的 Pipeline 调用
async fn resolve_answer(&self, question_id: u32, result: Bytes) {
    // 存储结果
    self.store_resolved_answer(question_id, result.clone());

    // 分发所有等待此结果的 Pipeline 调用
    if let Some(waiters) = self.take_pipeline_waiters(question_id) {
        for waiter in waiters {
            let capability = result.extract_field(&waiter.field_path);
            tokio::spawn(async move {
                capability.handle(waiter.call.method, waiter.payload).await;
            });
        }
    }
}
```

### 4.5 Go 端对应实现

```go
// ============= Go 伪代码 =============

// PipelineTarget 描述一个 Pipeline 调用的依赖
type PipelineTarget struct {
    QuestionID uint32   // 依赖的调用的 stream_id
    FieldPath  []string // 从结果中提取的字段路径
}

// CallOnPromise 创建引用另一个未完成调用结果的 Pipeline 调用
func (p *RpcPeer) CallOnPromise(
    promiseStreamID uint32,
    fieldPath []string,
    method string,
    payload []byte,
) ([]byte, error) {
    streamID := p.allocStreamID()

    metadata := map[string]string{
        ":pipeline-target": "true",
        ":pipeline-qid":    fmt.Sprintf("%d", promiseStreamID),
        ":pipeline-field":  strings.Join(fieldPath, "."),
    }

    call := &pb.Call{Method: method, Metadata: metadata}
    pc := p.insertPendingCall(streamID)

    p.sendHeaders(streamID, call)
    p.sendData(streamID, payload)
    p.sendTrailers(streamID, statusOK())

    return pc.Wait()
}
```

### 4.6 用户 API 设计

Pipeline 的最终目标是让用户代码简洁直观：

```rust
// ============= Rust 用户代码示例 =============

// 不使用 Pipeline：3 次 RTT
let dir = peer.call("FS/OpenDir", b"/home").await?;
let file = peer.call("Dir/Open", dir).await?;
let data = peer.call("File/Read", file).await?;

// 使用 Pipeline：1 次 RTT
let q1 = peer.call_async("FS/OpenDir", b"/home");          // 不等待
let q2 = peer.call_on_promise(q1.id(), vec![], "Dir/Open", b"a.txt");
let data = peer.call_on_promise(q2.id(), vec![], "File/Read", read_params).await?;
```

```go
// ============= Go 用户代码示例 =============

// 不使用 Pipeline：3 次 RTT
dir, _ := peer.Call("FS/OpenDir", []byte("/home"))
file, _ := peer.Call("Dir/Open", dir)
data, _ := peer.Call("File/Read", file)

// 使用 Pipeline：1 次 RTT
q1 := peer.CallAsync("FS/OpenDir", []byte("/home"))
q2 := peer.CallOnPromise(q1.ID(), nil, "Dir/Open", []byte("a.txt"))
data, _ := peer.CallOnPromise(q2.ID(), nil, "File/Read", readParams)
// 实际网络通信只发生一次
```

---

## 5. 分阶段实施路线图

### Phase 0：Pipeline 感知的调用分发

**目标**：服务端能识别 Pipeline 调用并正确排队/分发

**工作内容**：
- [ ] 定义 Pipeline metadata 保留键规范
- [ ] 服务端解析 `:pipeline-target` / `:pipeline-qid` / `:pipeline-field`
- [ ] 新增 `resolved_answers` 表，缓存已完成调用的结果
- [ ] 新增 `pipeline_waiters` 队列，Pipeline 调用在依赖未就绪时排队等待
- [ ] 当依赖调用完成时，自动分发等待的 Pipeline 调用
- [ ] 单元测试：两个依赖调用验证 Pipeline 分发

**不需要**：客户端 API 变更、Capability 系统

**估算**：Rust 3-5 天，Go 2-3 天

### Phase 1：客户端 Pipeline API

**目标**：客户端提供 `call_on_promise` / `CallOnPromise` API

**工作内容**：
- [ ] Rust：`call_on_promise()` 和 `call_async()` API
- [ ] Go：`CallOnPromise()` 和 `CallAsync()` API
- [ ] Node.js：`callOnPromise()` API（通过 napi-rs 绑定）
- [ ] 批量发送优化：检测连续 Pipeline 调用并合并为单次网络写入
- [ ] 跨语言互通测试：Rust 客户端 → Go 服务端的 Pipeline 调用
- [ ] 性能基准测试：对比 Pipeline vs 串行调用的延迟

**估算**：3-5 天

### Phase 2：轻量 Capability 引用

**目标**：支持将 RPC 结果中的"对象引用"传递给后续调用

**工作内容**：
- [ ] 设计 Capability ID 分配和引用计数机制
- [ ] Exports/Imports 表实现
- [ ] `Release` 消息支持（引用计数归零时释放远端资源）
- [ ] Capability 代理对象（允许对导入的 Capability 直接调用方法）
- [ ] 生命周期管理和 GC

**估算**：5-8 天

### Phase 3：高级特性（可选）

- [ ] Persistent Capabilities（SturdyRef）—— 可序列化/持久化的 Capability 引用
- [ ] Three-Party Handoff —— Peer A 将 Peer B 的 Capability 介绍给 Peer C
- [ ] Pipeline 链路自动优化 —— 编译器/运行时自动检测可 Pipeline 的调用链

---

## 6. 参考资料

- [Cap'n Proto RPC Protocol 规范](https://capnproto.org/rpc.html) —— 官方协议文档，包含 Promise Pipelining 和 Capability 模型的完整定义
- [Cap'n Proto C++ RPC 文档](https://capnproto.org/cxxrpc.html) —— C++ 实现中的 Level 0-4 支持状态
- [Cap'n Proto DeepWiki: RPC System](https://deepwiki.com/capnproto/capnproto/3.3-rpc-system) —— 四张核心表和 Promise 解析的深入分析
- [E Language CapTP 协议](https://erights.org/elib/distrib/captp/index.html) —— Cap'n Proto Capability 模型的理论基础
- [Cap'n Proto 源码: rpc.capnp](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/rpc.capnp) —— 协议定义的 Schema 源文件
- [Rust capnp-rpc crate](https://docs.rs/capnp-rpc) —— Rust 语言的 Cap'n Proto RPC 实现
- [Go capnp RPC package](https://pkg.go.dev/capnproto.org/go/capnp/v3/rpc) —— Go 语言的 Cap'n Proto RPC 实现
