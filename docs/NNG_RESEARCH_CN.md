# NNG 深度调研文档

> forpc 项目中 NNG 功能使用情况调研与未利用功能分析

**日期**: 2026-03-06
**作者**: GitHub Copilot

---

## 目录

1. [NNG 概述](#1-nng-概述)
2. [forpc 当前使用的 NNG 功能](#2-forpc-当前使用的-nng-功能)
3. [未使用的 NNG 功能](#3-未使用的-nng-功能)
4. [功能详细分析](#4-功能详细分析)
5. [建议与优先级](#5-建议与优先级)
6. [参考资料](#6-参考资料)

---

## 1. NNG 概述

NNG（nanomsg-next-generation）是一个轻量级、高性能的消息传递库，是 nanomsg 的继任者。它实现了可伸缩性协议（Scalability Protocols），提供了丰富的通信模式、传输层和配置选项。

### 1.1 NNG 核心架构

```
┌──────────────────────────────────────────────────┐
│                  应用层 (Application)              │
├──────────────────────────────────────────────────┤
│             协议层 (Scalability Protocols)         │
│   REQ/REP | PUB/SUB | PAIR | BUS | PUSH/PULL    │
│               SURVEYOR/RESPONDENT                 │
├──────────────────────────────────────────────────┤
│              传输层 (Transports)                   │
│   TCP | IPC | Inproc | WebSocket | TLS | ZeroTier │
├──────────────────────────────────────────────────┤
│           异步 I/O 框架 (AIO / Context)           │
├──────────────────────────────────────────────────┤
│              统计与监控 (Statistics)               │
└──────────────────────────────────────────────────┘
```

### 1.2 forpc 使用的 NNG 库

| 语言 | 库 | 版本 |
|------|----|------|
| Rust | `anng`（基于 nng-rs） | v0.1.3（Git 依赖） |
| Go | `mangos/v3` | v3.4.2 |
| Node.js | `anng`（通过 napi-rs 绑定） | 与 Rust 共享 |

---

## 2. forpc 当前使用的 NNG 功能

### 2.1 协议

| 功能 | 使用状态 | 说明 |
|------|---------|------|
| REQ/REP Raw（Dealer-Router） | ✅ 已使用 | Rust: `Req0Raw`/`Rep0Raw`，Go: `xreq`/`xrep` |

forpc 仅使用了 **REQ/REP 的 Raw 模式**（即 Dealer-Router 模式），用于实现自定义的请求-响应多路复用。

### 2.2 传输层

| 传输方式 | 使用状态 | 说明 |
|---------|---------|------|
| TCP (`tcp://`) | ✅ 已使用 | 主要的网络传输方式 |
| IPC (`ipc://`) | ✅ 已使用 | 进程间通信 |
| Inproc (`inproc://`) | ✅ 已使用 | 仅用于测试 |

### 2.3 消息操作

| 功能 | 使用状态 | 说明 |
|------|---------|------|
| Message Header/Body 分离 | ✅ 已使用 | 4 字节 request ID 放在 header |
| `Message::with_capacity()` | ✅ 已使用 | 预分配消息缓冲区 |
| Socket Clone（多接收者） | ✅ 已使用 | 用于并发收发 |

### 2.4 连接管理

| 功能 | 使用状态 | 说明 |
|------|---------|------|
| 异步 Dial/Listen | ✅ 已使用 | 所有语言实现 |
| 手动重试逻辑 | ✅ 已使用 | 默认 10 次重试，100ms 间隔 |
| mpsc 通道缓冲 | ✅ 已使用 | 256 容量的发送/接收通道 |

### 2.5 当前未配置的 Socket 选项

> **重要发现**：forpc 目前 **没有配置任何 NNG Socket 选项**，完全依赖 NNG 的默认值。

---

## 3. 未使用的 NNG 功能

以下是 NNG 提供但 forpc 尚未利用的完整功能清单：

### 3.1 功能总览

| 类别 | 未使用功能 | 对 forpc 的潜在价值 |
|------|-----------|-------------------|
| **协议** | PUB/SUB, PAIR, BUS, PUSH/PULL, SURVEYOR/RESPONDENT | ⭐⭐ 中等 |
| **传输层** | WebSocket, TLS, ZeroTier | ⭐⭐⭐ 高 |
| **Socket 选项** | 超时、缓冲区、重连、Keepalive 等 | ⭐⭐⭐ 高 |
| **Context（上下文）** | nng_ctx 并发上下文 | ⭐⭐⭐ 高 |
| **Pipe 通知** | 连接/断连事件回调 | ⭐⭐⭐ 高 |
| **统计监控** | nng_stat 运行时统计 | ⭐⭐ 中等 |
| **零拷贝** | NNG_FLAG_ALLOC 零拷贝收发 | ⭐⭐ 中等 |
| **HTTP 服务** | NNG 内置 HTTP Server | ⭐ 低 |

---

## 4. 功能详细分析

### 4.1 WebSocket 传输（`ws://` / `wss://`）

**当前状态**：未使用

**功能说明**：
NNG 原生支持 WebSocket 传输，包括安全的 WSS（WebSocket over TLS）。这使得 NNG 消息可以穿越防火墙和 HTTP 代理，支持浏览器环境的集成。

**对 forpc 的价值**：
- 允许 Web 浏览器直接与 forpc 服务通信
- 穿越企业防火墙和 HTTP 代理
- 可通过标准 80/443 端口通信，降低部署难度

**使用方式**：
```
# 地址格式
ws://host:port/path
wss://host:port/path（TLS 加密）
```

**各语言支持情况**：
| 语言 | 库支持 | 备注 |
|------|--------|------|
| Rust (anng) | 需要确认 | 可能需要启用编译特性 |
| Go (mangos) | ✅ 支持 | `import _ "go.nanomsg.org/mangos/v3/transport/all"` |
| Node.js | 与 Rust 绑定共享 | 取决于 anng 的支持 |

---

### 4.2 TLS 加密传输（`tls+tcp://`）

**当前状态**：未使用

**功能说明**：
NNG 内置 TLS 1.2+ 支持（底层使用 mbedTLS），可为 TCP 连接提供端到端加密。

**对 forpc 的价值**：
- 加密进程间通信，防止数据窃听
- 支持客户端/服务端证书认证（mTLS）
- 适用于安全敏感的 IPC 场景

**配置示例**（C API 概念）：
```c
nng_tls_config *tls_cfg;
nng_tls_config_alloc(&tls_cfg, NNG_TLS_MODE_SERVER);
nng_tls_config_own_cert(tls_cfg, "server-cert.pem", "server-key.pem");
nng_listener_set_ptr(listener, NNG_OPT_TLS_CONFIG, tls_cfg);
```

**Go (mangos) 使用方式**：
```go
tlsCfg := &tls.Config{ /* 证书配置 */ }
sock.SetOption(mangos.OptionTLSConfig, tlsCfg)
```

---

### 4.3 Socket 选项配置

**当前状态**：完全未配置，使用默认值

以下是 forpc 可以利用的关键 Socket 选项：

#### 4.3.1 超时控制

| 选项 | 说明 | 默认值 | 建议 |
|------|------|--------|------|
| `NNG_OPT_SENDTIMEO` | 发送超时（毫秒） | 无限 | 设置合理超时避免阻塞 |
| `NNG_OPT_RECVTIMEO` | 接收超时（毫秒） | 无限 | 配合 forpc 的 `:timeout` 元数据 |

**分析**：forpc 目前通过应用层的 `tokio::time::timeout()`（Rust）、`select + time.After`（Go）和 `Promise.race()`（Node.js）实现超时，NNG 层面的超时可以作为补充的安全网。

#### 4.3.2 重连参数

| 选项 | 说明 | 默认值 | 建议 |
|------|------|--------|------|
| `NNG_OPT_RECONNMINT` | 最小重连间隔 | 100ms | 调整以匹配业务场景 |
| `NNG_OPT_RECONNMAXT` | 最大重连间隔 | 0（不退避） | 启用指数退避 |

**分析**：forpc 目前在应用层实现了手动重试逻辑（默认 10 次，100ms 间隔）。NNG 的内置重连机制支持指数退避，可以替代手动实现，减少代码复杂度。

#### 4.3.3 消息大小限制

| 选项 | 说明 | 默认值 | 建议 |
|------|------|--------|------|
| `NNG_OPT_RECVMAXSZ` | 最大接收消息大小 | 1MB | 根据业务需求调整 |

**分析**：设置消息大小上限可以防止恶意或错误的超大消息导致内存耗尽（DoS 防护）。

#### 4.3.4 缓冲区大小

| 选项 | 说明 | 默认值 | 建议 |
|------|------|--------|------|
| `NNG_OPT_SENDBUF` | 发送队列深度 | 取决于协议 | 高吞吐场景可增大 |
| `NNG_OPT_RECVBUF` | 接收队列深度 | 取决于协议 | 高吞吐场景可增大 |

#### 4.3.5 TCP 选项

| 选项 | 说明 | 默认值 | 建议 |
|------|------|--------|------|
| `NNG_OPT_TCP_KEEPALIVE` | TCP Keepalive | 关闭 | 启用以检测死连接 |
| `NNG_OPT_TCP_NODELAY` | 禁用 Nagle 算法 | 关闭 | RPC 场景建议启用降低延迟 |

**分析**：
- **TCP Keepalive**：对于长连接场景至关重要，可以及时发现断开的对端。forpc 目前没有检测连接健康状态的机制。
- **TCP Nodelay**：对于 RPC 这种请求-响应模式，禁用 Nagle 算法可以显著降低延迟。

---

### 4.4 NNG Context（协议上下文）

**当前状态**：未使用

**功能说明**：
`nng_ctx` 允许在单个 Socket 上创建多个独立的协议上下文，每个上下文维护自己的状态（请求 ID、超时、重试等）。这是 NNG 推荐的并发操作方式。

**对 forpc 的价值**：
- 替代 Raw 模式 + 手动请求 ID 管理
- 每个 Context 独立管理超时和重试
- 更安全的并发操作（无需自行维护状态）

**当前 forpc 实现 vs Context 方案对比**：

| 方面 | 当前方案（Raw Socket） | Context 方案 |
|------|---------------------|-------------|
| 请求 ID 管理 | 手动分配和追踪 | NNG 内部管理 |
| 并发模型 | 自行维护 mpsc 通道 | 每个 Context 独立状态 |
| 超时处理 | 应用层实现 | Context 级别可设 |
| 代码复杂度 | 较高 | 较低 |
| 灵活性 | 更高（完全自定义） | 受限于 NNG 协议逻辑 |

**注意**：forpc 使用 Raw 模式是有意为之——它允许自定义 Dealer-Router 多路复用语义和对等 RPC 模式。切换到 Context 模式可能需要重新设计协议层。

---

### 4.5 Pipe 通知（连接事件回调）

**当前状态**：未使用

**功能说明**：
`nng_pipe_notify` 允许注册回调函数，在连接建立或断开时收到通知。

**支持的事件**：
| 事件 | 说明 |
|------|------|
| `NNG_PIPE_EV_ADD_PRE` | 连接协商完成，尚未加入 Socket |
| `NNG_PIPE_EV_ADD_POST` | 连接已完全建立 |
| `NNG_PIPE_EV_REM_POST` | 连接已断开 |

**对 forpc 的价值**：
- **连接状态感知**：知道对端何时连接/断开
- **认证和授权**：在 `ADD_PRE` 阶段验证连接，拒绝非法客户端
- **资源清理**：在连接断开时清理关联的 stream 和资源
- **健康检查**：实时监控连接状态

**使用场景示例**：
```
连接建立 (ADD_POST):
  → 记录日志
  → 初始化对端资源
  → 触发握手流程

连接断开 (REM_POST):
  → 清理对端相关的所有 stream
  → 取消待处理的 RPC 请求
  → 通知上层应用
```

---

### 4.6 统计监控（nng_stat）

**当前状态**：未使用

**功能说明**：
NNG 提供运行时统计 API，可以获取 Socket、Listener、Dialer 和 Pipe 的实时状态数据。

**可获取的统计信息**：
| 统计项 | 说明 |
|--------|------|
| 活跃 Pipe 数量 | 当前连接数 |
| 发送/接收消息数 | 吞吐量统计 |
| 连接尝试/拒绝次数 | 连接健康状况 |
| 错误计数 | 错误频率统计 |

**对 forpc 的价值**：
- 运行时性能监控和诊断
- 帮助发现瓶颈和异常
- 可用于构建健康检查和告警机制

---

### 4.7 其他协议

**当前状态**：未使用

NNG 支持多种消息传递协议，forpc 仅使用了 REQ/REP（Raw 模式）。以下是其他可用协议：

| 协议 | 模式 | 潜在用途 |
|------|------|---------|
| **PUB/SUB** | 发布/订阅 | 事件广播、配置分发 |
| **PAIR** | 一对一双向 | 简化的点对点通信 |
| **PAIR v1 Polyamorous** | 一对多双向 | 多对端双向通信 |
| **BUS** | 多对多 | 消息总线、集群通信 |
| **PUSH/PULL** | 管道 | 任务分发、日志收集 |
| **SURVEYOR/RESPONDENT** | 调查 | 服务发现、健康检查 |

**分析**：
- **PUB/SUB** 可用于实现事件广播，比如配置变更通知。目前 forpc 的流式传输可以实现类似功能，但 PUB/SUB 更适合一对多场景。
- **SURVEYOR/RESPONDENT** 可用于实现服务发现——向所有连接的对端发起"调查"，收集可用的服务列表。
- **BUS** 适用于需要多个 forpc 实例互相通信的集群场景。

---

### 4.8 零拷贝消息处理

**当前状态**：部分使用（使用 Message API，但未使用 `NNG_FLAG_ALLOC`）

**功能说明**：
NNG 支持零拷贝的消息分配和传递。通过 `NNG_FLAG_ALLOC` 标志，可以让 NNG 直接管理消息缓冲区，避免不必要的内存拷贝。

**对 forpc 的价值**：
- 减少内存分配和拷贝开销
- 提升高吞吐场景下的性能
- 接收的消息缓冲区可直接复用于发送

---

### 4.9 NNG 内置 HTTP Server

**当前状态**：未使用（forpc 使用 Axum 实现 HTTP Proxy）

**功能说明**：
NNG 内置了一个轻量级 HTTP Server，可用于管理和控制接口。

**分析**：forpc 已经使用 Axum 实现了 HTTP Proxy，NNG 的内置 HTTP Server 在功能上较为有限，当前方案更为灵活，不建议替换。

---

## 5. 建议与优先级

### 5.1 高优先级（建议尽快采用）

| 功能 | 原因 | 实施难度 |
|------|------|---------|
| **TCP Keepalive** | 检测死连接，提高可靠性 | 🟢 低 |
| **TCP Nodelay** | 降低 RPC 延迟 | 🟢 低 |
| **Pipe 通知回调** | 连接状态感知、资源清理 | 🟡 中 |
| **消息大小限制** (`RECVMAXSZ`) | 安全防护（防 DoS） | 🟢 低 |

### 5.2 中优先级（根据需求评估）

| 功能 | 原因 | 实施难度 |
|------|------|---------|
| **NNG 内置重连机制** | 替代手动重试，支持指数退避 | 🟡 中 |
| **Socket 级超时** | 作为应用层超时的安全网 | 🟢 低 |
| **WebSocket 传输** | 支持浏览器和防火墙穿越 | 🟡 中 |
| **统计监控** | 运行时诊断和健康检查 | 🟡 中 |

### 5.3 低优先级（长期考虑）

| 功能 | 原因 | 实施难度 |
|------|------|---------|
| **TLS 加密** | 安全敏感场景的加密传输 | 🔴 高 |
| **PUB/SUB 协议** | 事件广播场景 | 🔴 高 |
| **SURVEYOR 协议** | 服务发现 | 🔴 高 |
| **Context 模式** | 需要重新设计协议层 | 🔴 高 |

### 5.4 不建议采用

| 功能 | 原因 |
|------|------|
| **NNG 内置 HTTP Server** | Axum 方案更灵活，已满足需求 |
| **ZeroTier 传输** | 使用场景有限，增加依赖复杂度 |
| **BUS/PAIR 协议** | 与 forpc 的 Dealer-Router 设计不兼容 |

---

## 6. 参考资料

- [NNG 官方文档](https://nng.nanomsg.org/man/tip/index.html)
- [NNG GitHub 仓库](https://github.com/nanomsg/nng)
- [NNG Socket 选项参考](https://nng.nanomsg.org/man/tip/nng_options.5.html)
- [NNG Context 文档](https://nng.nanomsg.org/man/tip/nng_ctx.5.html)
- [NNG Pipe 通知文档](https://nng.nanomsg.org/man/tip/nng_pipe_notify.3.html)
- [NNG WebSocket 传输](https://nng.nanomsg.org/man/tip/nng_ws.7.html)
- [NNG TLS 配置](https://nng.nanomsg.org/man/v1.10.0/nng_tls_config.5.html)
- [NNG 统计 API](https://nng.nanomsg.org/man/tip/nng_stat.5.html)
- [mangos v3 Go 文档](https://pkg.go.dev/go.nanomsg.org/mangos/v3)
- [mangos GitHub 仓库](https://github.com/nanomsg/mangos)
- [forpc 技术规范](./TECHNICAL_SPECIFICATION_CN.md)
