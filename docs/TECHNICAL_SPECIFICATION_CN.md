# Mini-RPC 技术规范文档

> 基于 gRPC 设计理念的高性能对等 IPC 通信框架

**版本**: 2.0  
**日期**: 2026-01-18  
**作者**: bytemain

---

## 目录

1. [概述](#1-概述)
2. [架构设计](#2-架构设计)
3. [协议规范](#3-协议规范)
4. [数据结构定义](#4-数据结构定义)
5. [RpcPeer 实现](#5-rpcpeer-实现)
6. [流式传输](#6-流式传输)
7. [错误处理](#7-错误处理)
8. [性能优化](#8-性能优化)
9. [测试方案](#9-测试方案)
10. [API 参考](#10-api-参考)
11. [附录](#附录)

---

## 1. 概述

### 1.1 项目背景

Mini-RPC 是一个轻量级、高性能的**对等 RPC（Peer-to-Peer RPC）**框架，其设计灵感来源于 Google 的 gRPC，专门针对 IPC 场景优化。与传统的 Client-Server 模式不同，Mini-RPC 采用对等架构，**连接双方都可以主动发起调用**。

| 组件 | gRPC | Mini-RPC |
|------|------|----------|
| **传输层** | HTTP/2 | NNG (nanomsg-next-gen) IPC |
| **序列化** | Protocol Buffers | Fory |
| **通信模式** | Client-Server | **Peer-to-Peer（对等）** |
| **调用方向** | 单向（Client→Server） | **双向（双方都可发起）** |

### 1.2 核心特性：对等 RPC

传统 RPC 框架中，Client 只能调用 Server，Server 只能被动响应。但在 IPC 场景中，经常需要：

- **服务端回调客户端**：处理请求时需要向客户端获取配置、凭证等信息
- **双向事件通知**：双方都可能产生需要通知对方的事件
- **代理场景**：代理服务需要与客户端进行多轮交互

```
┌─────────────────────────────────────────────────────────────────┐
│                    对等 RPC 调用示例                             │
└─────────────────────────────────────────────────────────────────┘

  Peer A (连接发起方)                           Peer B (连接接受方)
        │                                              │
        │  ────────── Call: Proxy/Send ───────────►   │
        │                                              │
        │                                              │  处理中...
        │                                              │  需要配置信息
        │                                              │
        │  ◄────────── Call: Client/GetConfig ─────   │  回调！
        │                                              │
        │  ────────── Response: {config} ──────────►  │
        │                                              │
        │                                              │  继续处理
        │                                              │
        │  ◄────────── Response: OK ───────────────   │
        │                                              │
```

### 1.3 设计目标

- **对等通信**: 连接双方地位平等，都可主动发起 RPC 调用
- **高性能**: 利用 NNG 的零拷贝特性和异步 I/O
- **低延迟**: IPC 本地通信，避免网络栈开销
- **流式支持**: 支持单向流和双向流
- **状态传播**: 完善的错误状态码和消息传递机制
- **元数据支持**: 类似 gRPC 的 metadata 机制
- **多路复用**: 单连接承载多个并发调用

### 1.4 技术栈

```
┌─────────────────────────────────────────────────┐
│                  应用层 (Application)            │
├─────────────────────────────────────────────────┤
│              Mini-RPC 框架层                     │
│   ┌─────────────────────────────────────────┐   │
│   │               RpcPeer                   │   │
│   │  - 注册服务 (register)                   │   │
│   │  - 发起调用 (call / stream)             │   │
│   │  - 双向通信                              │   │
│   └─────────────────────────────────────────┘   │
├─────────────────────────────────────────────────┤
│              协议层 (Protocol)                   │
│   ┌─────────────────────────────────────────┐   │
│   │  Packet { stream_id, kind, payload }    │   │
│   └─────────────────────────────────────────┘   │
├─────────────────────────────────────────────────┤
│              序列化层 (Serialization)            │
│   ┌─────────────────────────────────────────┐   │
│   │           Fory Serializer               │   │
│   └─────────────────────────────────────────┘   │
├─────────────────────────────────────────────────┤
│              传输层 (Transport)                  │
│   ┌─────────────────────────────────────────┐   │
│   │       NNG Dealer-Router          │   │
│   └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### 1.5 核心依赖

```toml
[dependencies]
anng = { git = "https://github.com/bytemain/nng-rs.git", branch = "main" }
tokio = { version = "1.49.0", features = ["full"] }
fory = "0.14"
```

---

## 2. 架构设计

### 2.1 对等架构

Mini-RPC 采用对等架构，每个 Peer 同时具备 Client 和 Server 的能力：

```
┌─────────────────────────────────────────────────────────────────┐
│                        对等 RPC 架构                             │
└─────────────────────────────────────────────────────────────────┘

    ┌─────────────────────────┐          ┌─────────────────────────┐
    │        Peer A           │          │        Peer B           │
    │    (连接发起方)          │          │    (连接接受方)          │
    │                         │          │                         │
    │  ┌───────────────────┐  │          │  ┌───────────────────┐  │
    │  │  注册的服务        │  │          │  │  注册的服务        │  │
    │  │  - Client/GetConfig│  │          │  │  - Proxy/Send     │  │
    │  │  - Client/OnEvent  │  │          │  │  - Proxy/Connect  │  │
    │  └───────────────────┘  │          │  └───────────────────┘  │
    │                         │          │                         │
    │  ┌───────────────────┐  │          │  ┌───────────────────┐  │
    │  │  发起调用能力      │──┼──────────┼─►│  处理调用         │  │
    │  │  call()           │  │          │  │                   │  │
    │  └───────────────────┘  │          │  └───────────────────┘  │
    │                         │          │                         │
    │  ┌───────────────────┐  │          │  ┌───────────────────┐  │
    │  │  处理调用         │◄─┼──────────┼──│  发起调用能力      │  │
    │  │                   │  │          │  │  call()           │  │
    │  └───────────────────┘  │          │  └───────────────────┘  │
    └─────────────────────────┘          └─────────────────────────┘
    
                      ◄───── 共享同一个 IPC 连接 ─────►
```

### 2.2 消息流转

```
┌─────────────────────────────────────────────────────────────────┐
│                        消息流转示意                              │
└─────────────────────────────────────────────────────────────────┘

                           Peer A                 Peer B
                             │                      │
    ┌────────────────────────┼──────────────────────┼────────────────────────┐
    │                        │                      │                        │
    │  应用层                 │                      │                 应用层  │
    │    │                   │                      │                   │    │
    │    ▼                   │                      │                   ▼    │
    │  ┌─────┐               │                      │               ┌─────┐  │
    │  │call │───┐           │                      │           ┌───│handler│ │
    │  └─────┘   │           │                      │           │   └─────┘  │
    │            ▼           │                      │           ▼            │
    │  ┌──────────────┐      │                      │      ┌──────────────┐  │
    │  │ 编码 Packet   │      │                      │      │ 解码 Packet   │  │
    │  │ stream_id=1  │      │                      │      │ stream_id=1  │  │
    │  └──────────────┘      │                      │      └──────────────┘  │
    │            │           │                      │           ▲            │
    │            ▼           │                      │           │            │
    │  ┌──────────────┐      │      HEADERS         │      ┌──────────────┐  │
    │  │   发送队列    │──────┼─────────────────────►┼──────│   接收队列    │  │
    │  └──────────────┘      │      DATA            │      └──────────────┘  │
    │            ▲           │      TRAILERS        │           │            │
    │            │           │◄─────────────────────┤           ▼            │
    │  ┌──────────────┐      │                      │      ┌──────────────┐  │
    │  │   接收队列    │◄─────┼──────────────────────┼──────│   发送队列    │  │
    │  └──────────────┘      │                      │      └──────────────┘  │
    │            │           │                      │           ▲            │
    │            ▼           │                      │           │            │
    │  ┌──────────────┐      │                      │      ┌──────────────┐  │
    │  │ 解码 Packet   │      │                      │      │ 编码 Packet   │  │
    │  │ stream_id=1  │      │                      │      │ stream_id=1  │  │
    │  └──────────────┘      │                      │      └──────────────┘  │
    │            │           │                      │           ▲            │
    │            ▼           │                      │           │            │
    │  ┌─────────┐           │                      │           │   ┌─────┐  │
    │  │ pending │◄──────────┼──────────────────────┼───────────┘   │reply│  │
    │  │ resolve │           │                      │               └─────┘  │
    │  └─────────┘           │                      │                        │
    │                        │                      │                        │
    └────────────────────────┼──────────────────────┼────────────────────────┘
                             │                      │
```

### 2.3 调用生命周期

**场景：Peer A 调用 Peer B，Peer B 在处理过程中回调 Peer A**

```
Peer A (连接发起方)                                    Peer B (连接接受方)
     │                                                       │
     │  ══════════ 调用 1: Proxy/Send (stream_id=1) ══════►  │
     │  ─────────── HEADERS ────────────────────────────►    │
     │  ─────────── DATA ───────────────────────────────►    │
     │  ─────────── TRAILERS (EOS) ─────────────────────►    │
     │                                                       │
     │                                                       │  处理 Proxy/Send...
     │                                                       │  需要客户端配置
     │                                                       │
     │  ◄══════════ 调用 2: Client/GetConfig (stream_id=2) ══│
     │  ◄─────────── HEADERS ────────────────────────────    │
     │  ◄─────────── DATA ───────────────────────────────    │
     │  ◄─────────── TRAILERS (EOS) ─────────────────────    │
     │                                                       │
     │  处理 GetConfig...                                     │
     │                                                       │
     │  ══════════ 响应 2: GetConfig Result ════════════════► │
     │  ─────────── HEADERS ────────────────────────────►    │
     │  ─────────── DATA {config} ──────────────────────►    │
     │  ─────────── TRAILERS (OK) ──────────────────────►    │
     │                                                       │
     │                                                       │  继续处理 Proxy/Send
     │                                                       │
     │  ◄══════════ 响应 1: Proxy/Send Result ═══════════════ │
     │  ◄─────────── HEADERS ────────────────────────────    │
     │  ◄─────────── DATA {result} ──────────────────────    │
     │  ◄─────────── TRAILERS (OK) ──────────────────────    │
     │                                                       │
```

---

## 3. 协议规范

### 3.1 帧类型定义

Mini-RPC 定义三种帧类型，对应 gRPC 的 HTTP/2 帧：

| 帧类型 | Kind 值 | 用途 | gRPC 对应 |
|--------|---------|------|-----------|
| **HEADERS** | `0` | 发起流，携带方法名和元数据 | HEADERS frame |
| **DATA** | `1` | 传输消息负载 | DATA frame |
| **TRAILERS** | `2` | 结束流，携带状态码和状态消息 | HEADERS frame (with END_STREAM) |

### 3.2 帧格式

```
┌─────────────────────────────────────────────────────────┐
│                      Packet                             │
├─────────────┬─────────────┬─────────────────────────────┤
│  stream_id  │    kind     │          payload            │
│   (4 bytes) │  (1 byte)   │       (variable length)     │
│    u32      │     u8      │          Vec<u8>            │
└─────────────┴─────────────┴─────────────────────────────┘
```

### 3.3 Payload 结构

#### 3.3.1 HEADERS Payload

```rust
// 当 kind = 0 (HEADERS) 时
struct Call {
    method: String,                      // 方法名，如 "Proxy/Send"
    metadata: HashMap<String, String>,   // 元数据键值对
}
```

**元数据保留键**:
| 键 | 描述 |
|----|------|
| `:timeout` | 请求超时时间（毫秒） |
| `:compression` | 压缩算法（none/gzip/lz4） |
| `:content-type` | 负载类型（默认 fory） |
| `:trace-id` | 分布式追踪 ID |

#### 3.3.2 DATA Payload

```rust
// 当 kind = 1 (DATA) 时
// payload 直接存储 Fory 序列化后的用户消息
payload: Vec<u8>  // Fory.serialize(&user_message)
```

#### 3.3.3 TRAILERS Payload

```rust
// 当 kind = 2 (TRAILERS) 时
struct Status {
    code: u32,       // 状态码
    message: String, // 状态描述
}
```

### 3.4 Stream ID 分配规则

对等模式下，Stream ID 根据**连接角色**分配：

```
┌─────────────────────────────────────────────────────────┐
│              Stream ID 分配（对等模式）                   │
├─────────────────────────────────────────────────────────┤
│  连接发起方（Initiator）发起的流: 奇数 (1, 3, 5, 7, ...) │
│  连接接受方（Acceptor）发起的流:  偶数 (2, 4, 6, 8, ...) │
│  Stream ID = 0: 控制流（保留）                          │
└─────────────────────────────────────────────────────────┘
```

**为什么这样设计**：
- 避免双方同时发起调用时 Stream ID 冲突
- 通过 ID 奇偶性可以判断调用发起方
- 无需额外的 ID 协商机制

---

## 4. 数据结构定义

### 4.1 核心数据结构

```rust
// =============================================================================
// 文件: src/rpc/protocol.rs
// =============================================================================

use fory::ForyObject;
use std::collections::HashMap;

/// 帧类型常量
pub mod frame_kind {
    pub const HEADERS: u8 = 0;
    pub const DATA: u8 = 1;
    pub const TRAILERS: u8 = 2;
}

/// 传输层数据包
/// 
/// 所有 RPC 通信都通过此结构进行封装
#[derive(ForyObject, Debug, Clone)]
pub struct Packet {
    /// 流标识符，用于多路复用
    /// - 奇数: 连接发起方发起的调用
    /// - 偶数: 连接接受方发起的调用
    pub stream_id: u32,
    
    /// 帧类型: 0=HEADERS, 1=DATA, 2=TRAILERS
    pub kind: u8,
    
    /// 负载数据，根据 kind 解释
    pub payload: Vec<u8>,
}

impl Packet {
    /// 创建 HEADERS 帧
    pub fn headers(stream_id: u32, call: &Call, fory: &mut Fory) -> Result<Self, Error> {
        Ok(Self {
            stream_id,
            kind: frame_kind::HEADERS,
            payload: fory.serialize(call)?,
        })
    }
    
    /// 创建 DATA 帧
    pub fn data(stream_id: u32, payload: Vec<u8>) -> Self {
        Self {
            stream_id,
            kind: frame_kind::DATA,
            payload,
        }
    }
    
    /// 创建 TRAILERS 帧
    pub fn trailers(stream_id: u32, status: &Status, fory: &mut Fory) -> Result<Self, Error> {
        Ok(Self {
            stream_id,
            kind: frame_kind::TRAILERS,
            payload: fory.serialize(status)?,
        })
    }
    
    /// 判断是否为流结束帧
    pub fn is_end_of_stream(&self) -> bool {
        self.kind == frame_kind::TRAILERS
    }
    
    /// 判断是否由连接发起方发起
    pub fn is_from_initiator(&self) -> bool {
        self.stream_id % 2 == 1
    }
}

/// RPC 调用信息（HEADERS payload）
#[derive(ForyObject, Debug, Clone)]
pub struct Call {
    /// 方法全名，格式: "ServiceName/MethodName"
    pub method: String,
    
    /// 元数据键值对
    pub metadata: HashMap<String, String>,
}

impl Call {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
    
    /// 获取超时设置（毫秒）
    pub fn timeout_ms(&self) -> Option<u64> {
        self.metadata.get(":timeout").and_then(|v| v.parse().ok())
    }
}

/// RPC 状态（TRAILERS payload）
#[derive(ForyObject, Debug, Clone)]
pub struct Status {
    /// 状态码
    pub code: u32,
    
    /// 状态消息
    pub message: String,
}

impl Status {
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
    
    pub fn ok() -> Self {
        Self { code: StatusCode::OK, message: "OK".into() }
    }
    
    pub fn cancelled(message: impl Into<String>) -> Self {
        Self { code: StatusCode::CANCELLED, message: message.into() }
    }
    
    pub fn unknown(message: impl Into<String>) -> Self {
        Self { code: StatusCode::UNKNOWN, message: message.into() }
    }
    
    pub fn internal(message: impl Into<String>) -> Self {
        Self { code: StatusCode::INTERNAL, message: message.into() }
    }
    
    pub fn is_ok(&self) -> bool {
        self.code == StatusCode::OK
    }
}
```

### 4.2 状态码定义

```rust
// =============================================================================
// 文件: src/rpc/status.rs
// =============================================================================

/// gRPC 兼容状态码
/// 
/// 参考: https://grpc.io/docs/guides/status-codes/
pub struct StatusCode;

impl StatusCode {
    /// 成功
    pub const OK: u32 = 0;
    
    /// 操作被取消（通常由调用者取消）
    pub const CANCELLED: u32 = 1;
    
    /// 未知错误
    pub const UNKNOWN: u32 = 2;
    
    /// 客户端指定了无效参数
    pub const INVALID_ARGUMENT: u32 = 3;
    
    /// 操作超时
    pub const DEADLINE_EXCEEDED: u32 = 4;
    
    /// 请求的实体不存在
    pub const NOT_FOUND: u32 = 5;
    
    /// 要创建的实体已存在
    pub const ALREADY_EXISTS: u32 = 6;
    
    /// 调用者没有权限执行该操作
    pub const PERMISSION_DENIED: u32 = 7;
    
    /// 资源已耗尽
    pub const RESOURCE_EXHAUSTED: u32 = 8;
    
    /// 操作被拒绝（前置条件检查失败）
    pub const FAILED_PRECONDITION: u32 = 9;
    
    /// 操作被中止
    pub const ABORTED: u32 = 10;
    
    /// 操作超出有效范围
    pub const OUT_OF_RANGE: u32 = 11;
    
    /// 操作未实现
    pub const UNIMPLEMENTED: u32 = 12;
    
    /// 内部错误
    pub const INTERNAL: u32 = 13;
    
    /// 服务不可用
    pub const UNAVAILABLE: u32 = 14;
    
    /// 数据丢失或损坏
    pub const DATA_LOSS: u32 = 15;
    
    /// 未认证
    pub const UNAUTHENTICATED: u32 = 16;
}

impl StatusCode {
    /// 获取状态码的文本描述
    pub fn description(code: u32) -> &'static str {
        match code {
            0 => "OK",
            1 => "CANCELLED",
            2 => "UNKNOWN",
            3 => "INVALID_ARGUMENT",
            4 => "DEADLINE_EXCEEDED",
            5 => "NOT_FOUND",
            6 => "ALREADY_EXISTS",
            7 => "PERMISSION_DENIED",
            8 => "RESOURCE_EXHAUSTED",
            9 => "FAILED_PRECONDITION",
            10 => "ABORTED",
            11 => "OUT_OF_RANGE",
            12 => "UNIMPLEMENTED",
            13 => "INTERNAL",
            14 => "UNAVAILABLE",
            15 => "DATA_LOSS",
            16 => "UNAUTHENTICATED",
            _ => "UNKNOWN_STATUS_CODE",
        }
    }
}
```

### 4.3 错误类型定义

```rust
// =============================================================================
// 文件: src/rpc/error.rs
// =============================================================================

use std::fmt;
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// RPC 错误类型
#[derive(Debug)]
pub struct RpcError {
    /// 状态码
    pub code: u32,
    /// 错误消息
    pub message: String,
    /// 错误来源（可选）
    pub source: Option<BoxError>,
}

impl RpcError {
    pub fn new(code: u32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }
    
    pub fn from_status(status: Status) -> Self {
        Self {
            code: status.code,
            message: status.message,
            source: None,
        }
    }
    
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }
    
    /// 判断是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.code,
            StatusCode::UNAVAILABLE | StatusCode::RESOURCE_EXHAUSTED | StatusCode::ABORTED
        )
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RpcError {{ code: {} ({}), message: \"{}\" }}",
            self.code,
            StatusCode::description(self.code),
            self.message
        )
    }
}

impl std::error::Error for RpcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// RPC 操作结果类型
pub type RpcResult<T> = Result<T, RpcError>;
```

---

## 5. RpcPeer 实现

### 5.1 RpcPeer 结构

```rust
// =============================================================================
// 文件: src/rpc/peer.rs
// =============================================================================

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot, RwLock};

/// 服务处理器类型
/// 
/// 处理器接收请求和 Peer 引用，可以在处理过程中回调对方
pub type BoxedHandler = Arc<
    dyn Fn(Request, Arc<RpcPeer>) -> Pin<Box<dyn Future<Output = Response> + Send>> + Send + Sync
>;

/// 对等 RPC 节点
/// 
/// 每个 Peer 同时具备发起调用和处理调用的能力
pub struct RpcPeer {
    /// 底层传输（双向）
    transport: Arc<dyn Transport>,
    
    /// 本端注册的服务处理器
    handlers: Arc<RwLock<HashMap<String, BoxedHandler>>>,
    
    /// 待处理的出站调用响应: stream_id -> response_sender
    pending_calls: Arc<Mutex<HashMap<u32, PendingCall>>>,
    
    /// 活跃的入站流状态: stream_id -> stream_state
    inbound_streams: Arc<Mutex<HashMap<u32, StreamState>>>,
    
    /// Stream ID 生成器
    next_stream_id: AtomicU32,
    
    /// 是否是连接发起方（决定 Stream ID 奇偶）
    is_initiator: bool,

    /// 协议序列化器（内部固定启用 compatible+xlang）
    proto_fory: Arc<Mutex<Fory>>,

    /// 用户序列化器（默认启用 compatible+xlang；建议用 namespace 注册类型）
    user_fory: Arc<Mutex<Fory>>,
    
    /// 是否正在运行
    running: Arc<AtomicBool>,
}

/// 待处理的出站调用
struct PendingCall {
    /// 一元调用响应发送器
    tx: Option<oneshot::Sender<RpcResult<Bytes>>>,
    /// 流式数据接收通道（用于流式响应）
    stream_tx: Option<mpsc::Sender<Packet>>,
    /// 一元响应 DATA 暂存
    unary_buffer: Option<Bytes>,
}

/// 入站流状态
struct StreamState {
    tx: mpsc::Sender<Packet>,
}

/// 请求上下文
pub struct Request {
    /// 方法名
    pub method: String,
    /// 元数据
    pub metadata: HashMap<String, String>,
    /// 请求负载（一元调用）
    pub payload: Option<Bytes>,
    /// 流式接收器（流式调用，接收 DATA/TRAILERS）
    pub stream: Option<mpsc::Receiver<Packet>>,
    /// 流 ID（用于流式响应）
    pub stream_id: u32,
}

/// 响应
pub struct Response {
    /// 响应元数据
    pub metadata: HashMap<String, String>,
    /// 响应负载（一元调用）
    pub payload: Option<Bytes>,
    /// 状态
    pub status: Status,
}

impl Response {
    pub fn ok(payload: Bytes) -> Self {
        Self {
            metadata: HashMap::new(),
            payload: Some(payload),
            status: Status::ok(),
        }
    }
    
    pub fn error(status: Status) -> Self {
        Self {
            metadata: HashMap::new(),
            payload: None,
            status,
        }
    }
    
    pub fn error_with_code(code: u32, message: impl Into<String>) -> Self {
        Self::error(Status::new(code, message))
    }
}
```

### 5.2 创建与连接

当前仓库实现中，`RpcPeer::connect/connect_with_retry` 底层使用 `transport::nng::ClientTransport`；如需更低层控制，也可直接 `ClientTransport::new(...) + RpcPeer::new(...)`。

当前实现默认启用 Fory 的 `compatible(true) + xlang(true)`，并推荐业务消息使用 `register_type_by_namespace::<T>(namespace, type_name)` 做类型注册，以避免跨语言/跨版本不一致问题。

```rust
impl RpcPeer {
    /// 作为连接发起方创建 Peer（主动连接）
    /// 
    /// # 参数
    /// - `url`: 目标地址，如 "ipc:///tmp/service.ipc" 或 "tcp://127.0.0.1:8080"
    /// 
    /// # 示例
    /// ```rust
    /// let peer = RpcPeer::connect("ipc:///tmp/proxy.ipc").await?;
    /// 
    /// // 注册本端服务（供对方回调）
    /// peer.register("Client/GetConfig", |req, _peer| async move {
    ///     Response::ok(serialize(&config)?)
    /// }).await;
    /// 
    /// // 调用对方服务
    /// let result: ProxyResponse = peer.call("Proxy/Send", request).await?;
    /// ```
    pub async fn connect(url: &str) -> RpcResult<Arc<Self>> {
        let transport = ClientTransport::new(url).await
            .map_err(|e| RpcError::new(StatusCode::UNAVAILABLE, e.to_string()))?;
        
        Ok(Arc::new(Self::new(transport, true))) // is_initiator = true
    }
    
    /// 作为连接发起方创建 Peer（带重试）
    pub async fn connect_with_retry(url: &str, max_retries: u32) -> RpcResult<Arc<Self>> {
        let transport = ClientTransport::new_with_retry(
            url,
            max_retries,
            Duration::from_millis(100),
        )
        .await
        .map_err(|e| RpcError::new(StatusCode::UNAVAILABLE, e.to_string()))?;
        
        Ok(Arc::new(Self::new(transport, true))) // is_initiator = true
    }
    
    /// 内部构造函数
    pub fn new(transport: impl Transport + 'static, is_initiator: bool) -> Self {
        let mut proto_fory = Fory::default().compatible(true).xlang(true);
        proto_fory.register::<Call>(2).unwrap();
        proto_fory.register::<Status>(3).unwrap();
        let user_fory = Fory::default().compatible(true).xlang(true);
        
        Self {
            transport: Arc::new(transport),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            pending_calls: Arc::new(Mutex::new(HashMap::new())),
            inbound_streams: Arc::new(Mutex::new(HashMap::new())),
            // 发起方用奇数(1,3,5...)，接受方用偶数(2,4,6...)
            next_stream_id: AtomicU32::new(if is_initiator { 1 } else { 2 }),
            is_initiator,
            proto_fory: Arc::new(Mutex::new(proto_fory)),
            user_fory: Arc::new(Mutex::new(user_fory)),
            running: Arc::new(AtomicBool::new(true)),
        }
    }
    
    /// 分配新的 Stream ID
    fn alloc_stream_id(&self) -> u32 {
        self.next_stream_id.fetch_add(2, Ordering::Relaxed)
    }
}
```

### 5.3 监听器与接受连接

```rust
/// RPC 监听器
/// 
/// 用于接受入站连接
pub struct RpcListener {
    inner: Listener,
}

impl RpcListener {
    /// 绑定地址并开始监听
    /// 
    /// # 示例
    /// ```rust
    /// let listener = RpcListener::bind("ipc:///tmp/proxy.ipc").await?;
    /// 
    /// loop {
    ///     let peer = listener.accept().await?;
    ///     
    ///     // 为每个连接注册服务
    ///     peer.register("Proxy/Send", handle_proxy_send).await;
    ///     
    ///     // 启动处理循环
    ///     tokio::spawn(async move {
    ///         peer.serve().await
    ///     });
    /// }
    /// ```
    pub async fn bind(url: &str) -> RpcResult<Self> {
        let inner = Listener::bind(url).await
            .map_err(|e| RpcError::new(StatusCode::UNAVAILABLE, e.to_string()))?;
        Ok(Self { inner })
    }
    
    /// 接受一个连接，返回对等 Peer
    pub async fn accept(&self) -> RpcResult<Arc<RpcPeer>> {
        let transport = self.inner.accept().await
            .map_err(|e| RpcError::new(StatusCode::UNAVAILABLE, e.to_string()))?;
        
        Ok(Arc::new(RpcPeer::new(transport, false))) // is_initiator = false
    }
}
```

### 5.4 服务注册

```rust
impl RpcPeer {
    /// 注册服务处理器
    /// 
    /// 处理器可以接收 Peer 引用，在处理过程中回调对方
    /// 
    /// # 示例
    /// ```rust
    /// peer.register("Proxy/Send", |req, peer| async move {
    ///     // 处理过程中回调客户端获取配置
    ///     let config: Config = peer.call("Client/GetConfig", Empty {}).await?;
    ///     
    ///     // 使用配置继续处理
    ///     if config.enabled {
    ///         // ...
    ///         Response::ok(serialize(&result)?)
    ///     } else {
    ///         Response::error_with_code(StatusCode::FAILED_PRECONDITION, "Disabled")
    ///     }
    /// }).await;
    /// ```
    pub async fn register<F, Fut>(&self, method: &str, handler: F)
    where
        F: Fn(Request, Arc<RpcPeer>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = move |req: Request, peer: Arc<RpcPeer>| -> Pin<Box<dyn Future<Output = Response> + Send>> {
            Box::pin(handler(req, peer))
        };
        
        let mut handlers = self.handlers.write().await;
        handlers.insert(method.to_string(), Box::new(handler));
    }
    
    /// 注册一元调用处理器（简化版，自动处理序列化）
    pub async fn register_unary<Req, Resp, F, Fut>(&self, method: &str, handler: F)
    where
        Req: Serializer + ForyDefault + Send + Sync + 'static,
        Resp: Serializer + ForyDefault + Send + Sync + 'static,
        F: Fn(Req, HashMap<String, String>, Arc<RpcPeer>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = RpcResult<Resp>> + Send + 'static,
    {
        let fory = self.user_fory.clone();
        
        self.register(method, move |req: Request, peer: Arc<RpcPeer>| {
            let fory = fory.clone();
            let handler = handler.clone();
            
            async move {
                // 反序列化请求
                let mut payload = Bytes::new();
                if let Some(mut rx) = req.stream {
                    while let Some(packet) = rx.recv().await {
                        if packet.kind == frame_kind::DATA {
                            payload = packet.payload;
                        }
                    }
                } else if let Some(p) = req.payload {
                    payload = p;
                }
                if payload.is_empty() {
                    return Response::error_with_code(StatusCode::INVALID_ARGUMENT, "Missing payload");
                }
                
                let request: Req = {
                    let f = fory.lock().await;
                    let mut reader = fory::Reader::new(&payload);
                    match f.deserialize_from(&mut reader) {
                        Ok(r) => r,
                        Err(e) => return Response::error_with_code(
                            StatusCode::INVALID_ARGUMENT,
                            format!("Deserialize error: {}", e),
                        ),
                    }
                };
                
                // 调用处理器
                match handler(request, req.metadata, peer).await {
                    Ok(response) => {
                        let f = fory.lock().await;
                        match f.serialize(&response) {
                            Ok(bytes) => Response::ok(Bytes::from(bytes)),
                            Err(e) => Response::error(Status::internal(e.to_string())),
                        }
                    }
                    Err(e) => Response::error(Status::new(e.code, e.message)),
                }
            }
        }).await;
    }
}
```

### 5.5 发起调用

```rust
impl RpcPeer {
    /// 一元调用
    /// 
    /// # 示例
    /// ```rust
    /// let response: UserInfo = peer.call("User/GetInfo", GetUserRequest { 
    ///     user_id: 123 
    /// }).await?;
    /// ```
    pub async fn call<Req, Resp>(&self, method: &str, request: Req) -> RpcResult<Resp>
    where
        Req: Serializer + Send + Sync + 'static,
        Resp: Serializer + ForyDefault + Send + Sync + 'static,
    {
        self.call_with_metadata(method, request, HashMap::new()).await
    }
    
    /// 带元数据的一元调用
    pub async fn call_with_metadata<Req, Resp>(
        &self,
        method: &str,
        request: Req,
        metadata: HashMap<String, String>,
    ) -> RpcResult<Resp>
    where
        Req: Serializer + Send + Sync + 'static,
        Resp: Serializer + ForyDefault + Send + Sync + 'static,
    {
        let stream_id = self.alloc_stream_id();
        
        // 1. 发送 HEADERS
        let call = Call {
            method: method.to_string(),
            metadata,
        };
        {
            let mut fory = self.proto_fory.lock().await;
            self.send_packet(Packet::headers(stream_id, &call, &mut fory)?).await?;
        }
        
        // 2. 发送 DATA
        let payload = {
            let fory = self.user_fory.lock().await;
            Bytes::from(fory.serialize(&request)
                .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?
            )
        };
        self.send_packet(Packet::data(stream_id, payload)).await?;
        
        // 3. 发送 TRAILERS（表示本端发送完毕）
        let eos_status = Status::ok();
        {
            let mut fory = self.proto_fory.lock().await;
            self.send_packet(Packet::trailers(stream_id, &eos_status, &mut fory)?).await?;
        }
        
        // 4. 注册 pending 并等待响应
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_calls.lock().await;
            pending.insert(stream_id, PendingCall { 
                tx: Some(tx), 
                stream_tx: None 
            });
        }
        
        let response_payload = rx.await
            .map_err(|_| RpcError::new(StatusCode::CANCELLED, "Call cancelled"))??;
        
        // 5. 反序列化响应
        let fory = self.user_fory.lock().await;
        let mut reader = fory::Reader::new(&response_payload);
        fory.deserialize_from(&mut reader)
            .map_err(|e| RpcError::new(StatusCode::INTERNAL, format!("Deserialize error: {}", e)))
    }
    
    /// 发送数据包
    async fn send_packet(&self, packet: Packet) -> RpcResult<()> {
        let bytes = packet.encode()
            .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;
        
        self.transport.send(bytes).await
            .map_err(|e| RpcError::new(StatusCode::UNAVAILABLE, e.to_string()))?;
        
        Ok(())
    }
}
```

### 5.6 消息处理循环

```rust
impl RpcPeer {
    pub async fn serve(self: &Arc<Self>) -> RpcResult<()> {
        while self.running.load(Ordering::Relaxed) {
            let bytes = match self.transport.recv().await {
                Ok(b) => b,
                Err(e) => {
                    if self.running.load(Ordering::Relaxed) {
                        return Err(RpcError::new(StatusCode::UNAVAILABLE, e.to_string()));
                    }
                    break;
                }
            };

            let packet = Packet::decode(bytes)
                .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;

            let is_inbound = if self.is_initiator {
                packet.stream_id % 2 == 0
            } else {
                packet.stream_id % 2 == 1
            };

            if is_inbound {
                self.handle_inbound(packet).await?;
            } else {
                self.handle_outbound(packet).await?;
            }
        }
        
        Ok(())
    }
}
```

注：消息循环的完整处理（入站 HEADERS/DATA/TRAILERS、多连接分流、pending_calls、inbound_streams）以源码为准：参见 [peer.rs](file:///Users/artin/0Workspace/github.com/bytemain/mini-rpc/rust/src/rpc/peer.rs) 与 [protocol.rs](file:///Users/artin/0Workspace/github.com/bytemain/mini-rpc/rust/src/rpc/protocol.rs)。

### 5.7 完整使用示例

```rust
// =============================================================================
// 示例: 代理服务场景
// =============================================================================

use mini_rpc::{RpcPeer, RpcListener, Request, Response, RpcResult};

// ============= 消息定义 =============

#[derive(ForyObject)]
struct ProxySendRequest {
    target: String,
    data: Vec<u8>,
}

#[derive(ForyObject)]
struct ProxySendResponse {
    success: bool,
    bytes_sent: u64,
}

#[derive(ForyObject)]
struct GetConfigRequest {}

#[derive(ForyObject)]
struct ClientConfig {
    proxy_enabled: bool,
    max_connections: u32,
    timeout_ms: u64,
}

// ============= 客户端（连接发起方）=============

async fn run_client() -> RpcResult<()> {
    // 连接到代理服务
    let peer = RpcPeer::connect("ipc:///tmp/proxy.ipc").await?;

    peer.register_type_by_namespace::<ProxySendRequest>("mini_rpc.example", "ProxySendRequest").await?;
    peer.register_type_by_namespace::<ProxySendResponse>("mini_rpc.example", "ProxySendResponse").await?;
    peer.register_type_by_namespace::<GetConfigRequest>("mini_rpc.example", "GetConfigRequest").await?;
    peer.register_type_by_namespace::<ClientConfig>("mini_rpc.example", "ClientConfig").await?;
    
    // 注册本端服务（供代理服务回调）
    peer.register_unary("Client/GetConfig", |_req: GetConfigRequest, _meta, _peer| async move {
        Ok(ClientConfig {
            proxy_enabled: true,
            max_connections: 10,
            timeout_ms: 5000,
        })
    }).await;
    
    // 后台启动消息处理（接收回调）
    let peer_serve = peer.clone();
    tokio::spawn(async move {
        if let Err(e) = peer_serve.serve().await {
            eprintln!("Client peer error: {}", e);
        }
    });
    
    // 调用代理服务
    let response: ProxySendResponse = peer.call("Proxy/Send", ProxySendRequest {
        target: "example.com:443".into(),
        data: b"Hello, World!".to_vec(),
    }).await?;
    
    println!("Sent {} bytes, success: {}", response.bytes_sent, response.success);
    
    Ok(())
}

// ============= 服务端（连接接受方）=============

async fn run_server() -> RpcResult<()> {
    let listener = RpcListener::bind("ipc:///tmp/proxy.ipc").await?;
    println!("Proxy server listening...");
    
    loop {
        // 接受连接
        let peer = listener.accept().await?;

        peer.register_type_by_namespace::<ProxySendRequest>("mini_rpc.example", "ProxySendRequest").await?;
        peer.register_type_by_namespace::<ProxySendResponse>("mini_rpc.example", "ProxySendResponse").await?;
        peer.register_type_by_namespace::<GetConfigRequest>("mini_rpc.example", "GetConfigRequest").await?;
        peer.register_type_by_namespace::<ClientConfig>("mini_rpc.example", "ClientConfig").await?;
        
        // 注册代理服务
        peer.register_unary("Proxy/Send", |req: ProxySendRequest, _meta, peer| async move {
            println!("Received proxy request to: {}", req.target);
            
            // 【关键】处理过程中回调客户端获取配置
            let config: ClientConfig = peer.call("Client/GetConfig", GetConfigRequest {}).await?;
            
            if !config.proxy_enabled {
                return Err(RpcError::new(
                    StatusCode::FAILED_PRECONDITION,
                    "Proxy is disabled in client config",
                ));
            }
            
            println!("Client config: max_connections={}, timeout={}ms", 
                config.max_connections, config.timeout_ms);
            
            // 使用配置执行代理逻辑...
            let bytes_sent = req.data.len() as u64;
            
            Ok(ProxySendResponse {
                success: true,
                bytes_sent,
            })
        }).await;
        
        // 为每个连接启动处理循环
        tokio::spawn(async move {
            if let Err(e) = peer.serve().await {
                eprintln!("Connection error: {}", e);
            }
        });
    }
}

// ============= 主函数 =============

#[tokio::main]
async fn main() -> RpcResult<()> {
    // 启动服务端
    tokio::spawn(run_server());
    
    // 等待服务端启动
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 运行客户端
    run_client().await
}
```

---

## 6. 流式传输

### 6.1 流模式概述

Mini-RPC 支持四种 RPC 模式：

| 模式 | 调用方 | 被调用方 | 描述 |
|------|--------|----------|------|
| **Unary** | 单消息 | 单消息 | 传统请求-响应模式 |
| **Server Streaming** | 单消息 | 多消息流 | 被调用方返回数据流 |
| **Client Streaming** | 多消息流 | 单消息 | 调用方发送数据流 |
| **Bidirectional** | 多消息流 | 多消息流 | 全双工流 |

> **注意**：在对等 RPC 中，"Server" 和 "Client" 是相对的，指的是某次调用的发起方和接收方。

### 6.2 流状态机

```
                    ┌─────────────────────────────────────────┐
                    │              流状态机                    │
                    └─────────────────────────────────────────┘
                    
    ┌──────────┐     HEADERS      ┌───────────┐      DATA       ┌──────────┐
    │  IDLE    │ ───────────────► │  OPEN     │ ◄─────────────► │  OPEN    │
    │          │                  │ (SENDING) │                 │ (RECVING)│
    └──────────┘                  └───────────┘                 └──────────┘
                                        │                            │
                                        │ TRAILERS                   │ TRAILERS
                                        ▼                            ▼
                                  ┌───────────┐                ┌───────────┐
                                  │HALF_CLOSED│                │HALF_CLOSED│
                                  │ (LOCAL)   │                │ (REMOTE)  │
                                  └───────────┘                └───────────┘
                                        │                            │
                                        │ TRAILERS (from remote)     │ TRAILERS (send)
                                        │                            │
                                        ▼                            ▼
                                  ┌─────────────────────────────────────┐
                                  │              CLOSED                  │
                                  └─────────────────────────────────────┘
```

### 6.3 流式接口

```rust
/// 双向流句柄
pub struct BidiStream<Req, Resp> {
    stream_id: u32,
    peer: Arc<RpcPeer>,
    recv_rx: mpsc::Receiver<Packet>,
    _marker: std::marker::PhantomData<(Req, Resp)>,
}

impl<Req: Serializer + Send + Sync + 'static, Resp: Serializer + ForyDefault + Send + Sync + 'static> BidiStream<Req, Resp> {
    /// 发送消息
    pub async fn send(&self, message: Req) -> RpcResult<()> {
        let payload = {
            let fory = self.peer.user_fory.lock().await;
            Bytes::from(
                fory.serialize(&message)
                    .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?,
            )
        };
        
        self.peer.send_packet(Packet::data(self.stream_id, payload)).await
    }
    
    /// 接收消息
    pub async fn recv(&mut self) -> RpcResult<Option<Resp>> {
        match self.recv_rx.recv().await {
            Some(packet) => {
                match packet.kind {
                    frame_kind::DATA => {
                        let fory = self.peer.user_fory.lock().await;
                        let mut reader = Reader::new(&packet.payload);
                        let msg: Resp = fory
                            .deserialize_from(&mut reader)
                            .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;
                        Ok(Some(msg))
                    }
                    frame_kind::TRAILERS => {
                        let fory = self.peer.proto_fory.lock().await;
                        let mut reader = Reader::new(&packet.payload);
                        let status: Status = fory
                            .deserialize_from(&mut reader)
                            .map_err(|e| RpcError::new(StatusCode::INTERNAL, e.to_string()))?;
                        if status.is_ok() {
                            Ok(None)
                        } else {
                            Err(RpcError::from_status(status))
                        }
                    }
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
    
    /// 关闭发送端
    pub async fn close_send(&self) -> RpcResult<()> {
        let status = Status::ok();
        let mut fory = self.peer.proto_fory.lock().await;
        self.peer
            .send_packet(Packet::trailers(self.stream_id, &status, &mut fory)?)
            .await
    }
}

impl RpcPeer {
    /// 创建双向流
    pub async fn stream<Req, Resp>(
        self: &Arc<Self>,
        method: &str,
    ) -> RpcResult<BidiStream<Req, Resp>>
    where
        Req: Serializer + Send + Sync + 'static,
        Resp: Serializer + ForyDefault + Send + Sync + 'static,
    {
        let stream_id = self.alloc_stream_id();
        
        // 发送 HEADERS
        let call = Call::new(method);
        {
            let mut fory = self.proto_fory.lock().await;
            self.send_packet(Packet::headers(stream_id, &call, &mut fory)?).await?;
        }
        
        // 创建接收通道
        let (tx, rx) = mpsc::channel(32);
        
        // 注册 pending
        {
            let mut pending = self.pending_calls.lock().await;
            pending.insert(stream_id, PendingCall {
                tx: None,
                stream_tx: Some(tx),
            });
        }
        
        Ok(BidiStream {
            stream_id,
            peer: self.clone(),
            recv_rx: rx,
            _marker: std::marker::PhantomData,
        })
    }
}
```

### 6.4 流式示例：文件传输

```rust
// ============= 文件下载（服务端流式）=============

// 注册下载处理器
peer.register("File/Download", |req, peer| async move {
    let download_req: DownloadRequest = peer.user_deserialize(&req.payload.unwrap()).await?;
    
    // 打开文件
    let mut file = File::open(&download_req.path).await?;
    let mut buffer = [0u8; 4096];
    
    // 分块发送
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        
        let payload = peer.user_serialize(&DataChunk { data: buffer[..n].to_vec() }).await?;
        peer.send_stream_data(req.stream_id, payload).await?;
    }
    
    Response { metadata: HashMap::new(), payload: None, status: Status::ok() }
});

// 客户端下载
let mut stream = peer.stream::<DownloadRequest, DataChunk>("File/Download").await?;
stream.send(DownloadRequest { path: "/data/file.bin".into() }).await?;
stream.close_send().await?;

let mut file = File::create("output.bin").await?;
while let Some(chunk) = stream.recv().await? {
    file.write_all(&chunk.data).await?;
}


// ============= 实时聊天（双向流式）=============

// 服务端：使用 register 接收 Request.stream，并通过 send_stream_data 主动推送消息
peer.register("Chat/Connect", |req, peer| async move {
    let Some(mut rx) = req.stream else {
        return Response::error_with_code(StatusCode::INVALID_ARGUMENT, "missing stream");
    };
    while let Some(pkt) = rx.recv().await {
        if pkt.kind != frame_kind::DATA {
            continue;
        }
        let msg: ChatMessage = match peer.user_deserialize(&pkt.payload).await {
            Ok(m) => m,
            Err(e) => return Response::error_with_code(StatusCode::INVALID_ARGUMENT, e.to_string()),
        };
        let response = ChatMessage { text: format!("Echo: {}", msg.text) };
        let payload = match peer.user_serialize(&response).await {
            Ok(p) => p,
            Err(e) => return Response::error_with_code(StatusCode::INTERNAL, e.to_string()),
        };
        if peer.send_stream_data(req.stream_id, payload).await.is_err() {
            break;
        }
    }
    Response { metadata: HashMap::new(), payload: None, status: Status::ok() }
});

// 客户端：stream() 即双向流（send/recv）
let mut stream = peer.stream::<ChatMessage, ChatMessage>("Chat/Connect").await?;
for i in 0..10 {
    stream.send(ChatMessage { text: format!("Message {}", i) }).await?;
}
stream.close_send().await?;

while let Some(msg) = stream.recv().await? {
    println!("{}", msg.text);
}
```

---

## 7. 错误处理

### 7.1 错误传播流程

```
┌──────────────────────────────────────────────────────────────────┐
│                        错误传播流程                               │
└──────────────────────────────────────────────────────────────────┘

  Peer A (调用方)                                   Peer B (被调用方)
     │                                                    │
     │  ─────────── Call: Method ────────────────────►   │
     │                                                    │
     │                                                    │ Handler 执行
     │                                                    │    │
     │                                                    │    ▼
     │                                                    │ ┌─────────┐
     │                                                    │ │  Error  │
     │                                                    │ └────┬────┘
     │                                                    │      │
     │  ◄─────────── TRAILERS ────────────────────────── │◄─────┘
     │               {code: 5, message: "Not found"}     │
     │                                                    │
     ▼                                                    │
┌─────────┐                                               │
│RpcError │                                               │
│ code: 5 │                                               │
│ msg: .. │                                               │
└─────────┘
```

### 7.2 错误处理最佳实践

```rust
// 处理器中的错误处理
async fn handle_proxy_send(
    req: ProxySendRequest,
    _meta: HashMap<String, String>,
    peer: Arc<RpcPeer>,
) -> RpcResult<ProxySendResponse> {
    // 参数验证
    if req.target.is_empty() {
        return Err(RpcError::new(
            StatusCode::INVALID_ARGUMENT,
            "target cannot be empty",
        ));
    }
    
    // 回调客户端获取配置（可能失败）
    let config: ClientConfig = peer.call("Client/GetConfig", GetConfigRequest {})
        .await
        .map_err(|e| RpcError::new(
            StatusCode::FAILED_PRECONDITION,
            format!("Failed to get client config: {}", e),
        ))?;
    
    // 前置条件检查
    if !config.proxy_enabled {
        return Err(RpcError::new(
            StatusCode::FAILED_PRECONDITION,
            "Proxy is disabled",
        ));
    }
    
    // 业务逻辑
    let result = do_proxy_send(&req.target, &req.data).await
        .map_err(|e| match e {
            ProxyError::ConnectionFailed => RpcError::new(StatusCode::UNAVAILABLE, "Connection failed"),
            ProxyError::Timeout => RpcError::new(StatusCode::DEADLINE_EXCEEDED, "Request timeout"),
            _ => RpcError::new(StatusCode::INTERNAL, e.to_string()),
        })?;
    
    Ok(ProxySendResponse {
        success: true,
        bytes_sent: result.bytes_sent,
    })
}

// 调用方的错误处理
match peer.call::<_, ProxySendResponse>("Proxy/Send", request).await {
    Ok(response) => {
        println!("Success: {} bytes sent", response.bytes_sent);
    }
    Err(e) => {
        match e.code {
            StatusCode::INVALID_ARGUMENT => {
                println!("Invalid request: {}", e.message);
            }
            StatusCode::FAILED_PRECONDITION => {
                println!("Precondition failed: {}", e.message);
            }
            StatusCode::UNAVAILABLE | StatusCode::DEADLINE_EXCEEDED => {
                if e.is_retryable() {
                    // 实现重试逻辑
                    println!("Retryable error: {}", e.message);
                }
            }
            _ => {
                println!("RPC failed: {}", e);
            }
        }
    }
}
```

### 7.3 超时与取消

```rust
// 带超时的调用
let result = tokio::time::timeout(
    Duration::from_secs(5),
    peer.call::<_, Response>("Method", request),
).await;

match result {
    Ok(Ok(response)) => { /* 成功 */ }
    Ok(Err(rpc_error)) => { /* RPC 错误 */ }
    Err(_) => {
        println!("Request timed out");
        // 可以发送取消帧
    }
}

// 使用元数据设置超时
let response = peer.call_with_metadata(
    "Method",
    request,
    hashmap! { ":timeout".into() => "5000".into() },
).await?;
```

---

## 8. 性能优化

### 8.1 零拷贝设计

```rust
/// 零拷贝消息传递
pub struct ZeroCopyBuffer {
    inner: Arc<Vec<u8>>,
    offset: usize,
    len: usize,
}

impl ZeroCopyBuffer {
    pub fn new(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            inner: Arc::new(data),
            offset: 0,
            len,
        }
    }
    
    pub fn slice(&self, start: usize, end: usize) -> Self {
        Self {
            inner: self.inner.clone(),
            offset: self.offset + start,
            len: end - start,
        }
    }
    
    pub fn as_slice(&self) -> &[u8] {
        &self.inner[self.offset..self.offset + self.len]
    }
}
```

### 8.2 对象池

```rust
/// 缓冲区对象池
pub struct BufferPool {
    pool: Mutex<VecDeque<Vec<u8>>>,
    buffer_size: usize,
    max_pool_size: usize,
}

impl BufferPool {
    pub fn new(buffer_size: usize, max_pool_size: usize) -> Self {
        Self {
            pool: Mutex::new(VecDeque::with_capacity(max_pool_size)),
            buffer_size,
            max_pool_size,
        }
    }
    
    pub fn get(&self) -> Vec<u8> {
        let mut pool = self.pool.lock().unwrap();
        pool.pop_front().unwrap_or_else(|| Vec::with_capacity(self.buffer_size))
    }
    
    pub fn put(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            pool.push_back(buffer);
        }
    }
}
```

### 8.3 性能指标

| 场景 | 预期性能 |
|------|---------|
| 一元调用延迟 (IPC) | < 100μs |
| 一元调用延迟 (TCP localhost) | < 500μs |
| 吞吐量 (小消息 <1KB) | > 100K req/s |
| 吞吐量 (大消息 ~64KB) | > 1GB/s |
| 流式传输吞吐量 | 接近 IPC 带宽上限 |
| 回调延迟开销 | < 50μs |

---

## 9. 测试方案

### 9.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_packet_serialization() {
        let packet = Packet {
            stream_id: 1,
            kind: frame_kind::DATA,
            payload: Bytes::from_static(&[1, 2, 3, 4]),
        };
        
        let bytes = packet.clone().encode().unwrap();
        let decoded = Packet::decode(bytes).unwrap();
        
        assert_eq!(packet.stream_id, decoded.stream_id);
        assert_eq!(packet.kind, decoded.kind);
        assert_eq!(packet.payload, decoded.payload);
    }
    
    #[tokio::test]
    async fn test_stream_id_allocation() {
        // 发起方应该分配奇数 ID
        let initiator = RpcPeer::new(mock_transport(), true);
        assert_eq!(initiator.alloc_stream_id(), 1);
        assert_eq!(initiator.alloc_stream_id(), 3);
        assert_eq!(initiator.alloc_stream_id(), 5);
        
        // 接受方应该分配偶数 ID
        let acceptor = RpcPeer::new(mock_transport(), false);
        assert_eq!(acceptor.alloc_stream_id(), 2);
        assert_eq!(acceptor.alloc_stream_id(), 4);
        assert_eq!(acceptor.alloc_stream_id(), 6);
    }
}
```

### 9.2 集成测试

```rust
#[tokio::test]
async fn test_bidirectional_call() -> RpcResult<()> {
    let url = "ipc:///tmp/test_bidi.ipc";
    
    // 启动服务端
    let server_handle = tokio::spawn(async move {
        let listener = RpcListener::bind(url).await?;
        let peer = listener.accept().await?;
        
        // 服务端注册处理器，处理时会回调客户端
        peer.register_unary("Server/Process", |req: ProcessRequest, _meta, peer| async move {
            // 回调客户端获取附加信息
            let info: ClientInfo = peer.call("Client/GetInfo", Empty {}).await?;
            
            Ok(ProcessResponse {
                result: format!("Processed by {} for {}", info.name, req.data),
            })
        }).await;
        
        peer.serve().await
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 客户端
    let peer = RpcPeer::connect(url).await?;
    
    // 客户端注册回调处理器
    peer.register_unary("Client/GetInfo", |_req: Empty, _meta, _peer| async move {
        Ok(ClientInfo { name: "TestClient".into() })
    }).await;
    
    // 后台运行消息循环
    let peer_serve = peer.clone();
    tokio::spawn(async move { peer_serve.serve().await });
    
    // 调用服务端
    let response: ProcessResponse = peer.call("Server/Process", ProcessRequest {
        data: "test data".into(),
    }).await?;
    
    assert_eq!(response.result, "Processed by TestClient for test data");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_bidirectional_calls() -> RpcResult<()> {
    // 测试双方同时发起调用不会冲突
    let url = "ipc:///tmp/test_concurrent.ipc";
    
    let server_handle = tokio::spawn(async move {
        let listener = RpcListener::bind(url).await?;
        let peer = listener.accept().await?;
        
        peer.register_unary("Server/Echo", |req: EchoRequest, _meta, _peer| async move {
            Ok(EchoResponse { message: req.message })
        }).await;
        
        // 服务端也主动调用客户端
        let peer_call = peer.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _: EchoResponse = peer_call.call("Client/Echo", EchoRequest { 
                message: "from server".into() 
            }).await.unwrap();
        });
        
        peer.serve().await
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let peer = RpcPeer::connect(url).await?;
    
    peer.register_unary("Client/Echo", |req: EchoRequest, _meta, _peer| async move {
        Ok(EchoResponse { message: req.message })
    }).await;
    
    let peer_serve = peer.clone();
    tokio::spawn(async move { peer_serve.serve().await });
    
    // 客户端调用服务端
    let response: EchoResponse = peer.call("Server/Echo", EchoRequest {
        message: "from client".into(),
    }).await?;
    
    assert_eq!(response.message, "from client");
    
    Ok(())
}
```

### 9.3 性能测试

```rust
#[tokio::test]
async fn bench_bidirectional_latency() {
    let url = "ipc:///tmp/bench_bidi.ipc";
    
    tokio::spawn(async move {
        let listener = RpcListener::bind(url).await.unwrap();
        let peer = listener.accept().await.unwrap();
        
        peer.register_unary("Bench/Ping", |_: Empty, _meta, peer| async move {
            // 处理时回调客户端
            let _: Empty = peer.call("Client/Pong", Empty {}).await?;
            Ok(Empty {})
        }).await;
        
        peer.serve().await.unwrap();
    });
    
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let peer = RpcPeer::connect(url).await.unwrap();
    
    peer.register_unary("Client/Pong", |_: Empty, _meta, _peer| async move {
        Ok(Empty {})
    }).await;
    
    let peer_serve = peer.clone();
    tokio::spawn(async move { peer_serve.serve().await });
    
    // 预热
    for _ in 0..100 {
        let _: Empty = peer.call("Bench/Ping", Empty {}).await.unwrap();
    }
    
    // 测量（含回调的完整往返）
    let iterations = 10000;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let _: Empty = peer.call("Bench/Ping", Empty {}).await.unwrap();
    }
    
    let elapsed = start.elapsed();
    let avg_latency = elapsed / iterations;
    
    println!("Average round-trip latency (with callback): {:?}", avg_latency);
}
```

---

## 10. API 参考

### 10.1 RpcPeer

| 方法 | 描述 |
|------|------|
| `connect(url)` | 作为发起方连接 |
| `connect_with_retry(url, retries)` | 带重试连接 |
| `register(method, handler)` | 注册通用处理器 |
| `register_unary(method, handler)` | 注册一元处理器 |
| `call<Req, Resp>(method, request)` | 一元调用 |
| `call_with_metadata(...)` | 带元数据的一元调用 |
| `stream<Req, Resp>(method)` | 创建双向流 |
| `serve()` | 启动消息处理循环 |
| `close()` | 关闭连接 |

### 10.2 RpcListener

| 方法 | 描述 |
|------|------|
| `bind(url)` | 绑定地址监听 |
| `accept()` | 接受连接，返回 RpcPeer |

### 10.3 BidiStream

| 方法 | 描述 |
|------|------|
| `send(message)` | 发送消息 |
| `recv()` | 接收消息 |
| `close_send()` | 关闭发送端 |

---

## 附录

### A. 状态码速查表

| 码值 | 名称 | 描述 | 可重试 |
|------|------|------|--------|
| 0 | OK | 成功 | - |
| 1 | CANCELLED | 操作被取消 | 否 |
| 2 | UNKNOWN | 未知错误 | 视情况 |
| 3 | INVALID_ARGUMENT | 无效参数 | 否 |
| 4 | DEADLINE_EXCEEDED | 超时 | 是 |
| 5 | NOT_FOUND | 未找到 | 否 |
| 6 | ALREADY_EXISTS | 已存在 | 否 |
| 7 | PERMISSION_DENIED | 权限拒绝 | 否 |
| 8 | RESOURCE_EXHAUSTED | 资源耗尽 | 是 |
| 9 | FAILED_PRECONDITION | 前置条件失败 | 否 |
| 10 | ABORTED | 操作中止 | 是 |
| 11 | OUT_OF_RANGE | 超出范围 | 否 |
| 12 | UNIMPLEMENTED | 未实现 | 否 |
| 13 | INTERNAL | 内部错误 | 否 |
| 14 | UNAVAILABLE | 服务不可用 | 是 |
| 15 | DATA_LOSS | 数据丢失 | 否 |
| 16 | UNAUTHENTICATED | 未认证 | 否 |

### B. 元数据保留键

| 键 | 格式 | 描述 |
|----|------|------|
| `:timeout` | 数字 (ms) | 请求超时 |
| `:compression` | none/gzip/lz4 | 压缩算法 |
| `:content-type` | 字符串 | 负载类型 |
| `:authority` | 字符串 | 目标服务 |
| `:trace-id` | 字符串 | 追踪 ID |

### C. 文件结构

```
src/
├── lib.rs                 # 库入口
├── rpc/
│   ├── mod.rs            # 模块声明
│   ├── protocol.rs       # 协议定义 (Packet, Call, Status)
│   ├── status.rs         # 状态码定义
│   ├── error.rs          # 错误类型
│   ├── peer.rs           # RpcPeer 实现（核心）
│   ├── listener.rs       # RpcListener 实现
│   └── stream.rs         # 流式接口
├── transport/
│   ├── mod.rs
│   └── nng.rs            # NNG 传输层封装
└── util/
    ├── mod.rs
    ├── buffer_pool.rs    # 对象池
    └── zero_copy.rs      # 零拷贝工具
```

### D. 与 gRPC 概念映射

| gRPC 概念 | Mini-RPC 对应 |
|-----------|---------------|
| HTTP/2 Connection | IPC Socket |
| HTTP/2 Stream | stream_id |
| HEADERS frame | Packet(kind=0) |
| DATA frame | Packet(kind=1) |
| TRAILERS frame | Packet(kind=2) |
| Protobuf | Fory |
| Channel | **RpcPeer**（双向） |
| Server | **RpcPeer**（同时是 Server） |
| Client | **RpcPeer**（同时是 Client） |
| Metadata | Call.metadata |
| Status | Status |

### E. 对等 RPC vs 传统 RPC

| 特性 | 传统 RPC | Mini-RPC 对等 RPC |
|------|---------|-------------------|
| 调用方向 | Client → Server | 双向 |
| 角色固定 | 是 | 否（角色可互换） |
| 回调支持 | 需要额外机制 | 原生支持 |
| Stream ID | 客户端分配 | 按连接角色分配奇偶 |
| 适用场景 | Web API | IPC、插件系统、代理 |

---

**文档版本历史**

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-01-18 | 初始版本（Client-Server 模式） |
| 2.0 | 2026-01-18 | 重构为对等 RPC 架构 |

---

*© 2026 bytemain. Mini-RPC Project.*
