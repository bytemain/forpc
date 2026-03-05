# forpc 与主流 Peer RPC 框架差距分析

> **日期**: 2026-03-05
> **范围**: 将 forpc 与 gRPC、Cap'n Proto、Tarpc、DRPC、ConnectRPC、vscode-jsonrpc 等主流 RPC 框架进行对比，识别功能差距并给出优先级建议。

---

## 1. 项目现状总结

forpc 是一个基于 NNG 传输、使用 Protocol Buffers 做序列化的轻量级**对等 RPC/Streaming 框架**，支持 Rust / Go / Node.js 三语言互通。

### 已实现的核心能力

| 能力 | 状态 | 说明 |
|------|------|------|
| Unary 调用 | ✅ 已实现 | 单次请求/响应模式 |
| 双向流（Bidi Streaming） | ✅ 已实现 | 基于 stream_id 多路复用 |
| Raw 调用 | ✅ 已实现 | 不做 protobuf 编解码，直接传 bytes |
| 跨语言互通 | ✅ 已实现 | Rust ↔ Go ↔ Node.js 全矩阵测试 |
| Metadata 元数据 | ✅ 已实现 | 请求头键值对 `map<string, string>` |
| gRPC 兼容状态码 | ✅ 已实现 | 16 种标准状态码 |
| 多传输协议 | ✅ 已实现 | TCP / IPC / inproc |
| 连接重试 | ✅ 已实现 | `connect_with_retry` / `ConnectWithRetry` |
| Protobuf 代码生成 | ✅ 已实现 | Rust prost_build / Go protoc / Node pbjs |
| 对等通信 | ✅ 已实现 | 连接双方均可主动发起调用（核心差异化特性） |

---

## 2. 竞品对比矩阵

下表将 forpc 与主流 RPC 框架逐项对比：

| 特性 | forpc | gRPC | Cap'n Proto | Tarpc | DRPC | ConnectRPC | vscode-jsonrpc |
|------|-------|------|-------------|-------|------|------------|----------------|
| **调用模型** | | | | | | | |
| Unary 调用 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Server Streaming | ✅ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅（Notification） |
| Client Streaming | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ |
| Bidi Streaming | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ |
| 对等双向调用 | ✅ | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ |
| **可靠性** | | | | | | | |
| 超时/Deadline 执行 | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| Deadline 传播 | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 请求取消 | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 自动重试策略 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 连接 Keepalive | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 健康检查 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Wait-for-Ready | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **性能** | | | | | | | |
| 流控/背压 | ⚠️ | ✅ | ✅ | ❌ | ⚠️ | ✅ | ❌ |
| 消息压缩 | ❌ | ✅ | N/A（零拷贝） | ❌ | ❌ | ✅ | ❌ |
| 零拷贝序列化 | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 连接多路复用 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| **可扩展性** | | | | | | | |
| 拦截器/中间件 | ❌ | ✅ | ❌ | ❌ | ⚠️ | ✅ | ⚠️ |
| 服务发现 | ❌ | ✅（插件） | ❌ | ❌ | ❌ | ❌ | ❌ |
| 负载均衡 | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **安全** | | | | | | | |
| TLS/加密 | ❌ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ |
| 认证/鉴权 | ❌ | ✅ | ✅（Capability） | ❌ | ❌ | ✅ | ❌ |
| **可观测性** | | | | | | | |
| 分布式追踪 | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| 指标采集 | ❌ | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| Channelz | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **开发体验** | | | | | | | |
| Proto 服务代码生成 | ❌ | ✅ | ✅ | ✅（宏） | ✅ | ✅ | N/A |
| 类型安全 API | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 浏览器/Web 支持 | ❌ | ⚠️（gRPC-Web）| ❌ | ❌ | ❌ | ✅ | ✅ |
| **跨语言** | | | | | | | |
| 语言支持 | 3 种 | 10+ 种 | 5+ 种 | 仅 Rust | 仅 Go | Go + 扩展中 | JS/TS + .NET |
| 互通测试 | ✅ | ✅ | ✅ | N/A | N/A | ✅ | ✅ |

> **图例**：✅ 已实现　⚠️ 部分实现　❌ 未实现　N/A 不适用

---

## 3. 关键差距详细分析

### 3.1 超时/Deadline 执行（高优先级）

**现状**：`Call` 结构体中已定义 `:timeout` 元数据键和 `timeout_ms()` 解析方法，但运行时**完全不执行**超时。调用方会无限阻塞等待响应。

**gRPC 的做法**：
- 客户端设置 deadline，自动传播到下游服务
- 超时后自动返回 `DEADLINE_EXCEEDED` 错误
- 服务端可检查剩余时间，提前终止耗时操作

**建议**：
- Rust：在 `call()` 中用 `tokio::time::timeout()` 包裹响应等待
- Go：在 `Call()` 中使用 `context.WithTimeout()` + `select` 超时控制
- Node：使用 `Promise.race()` 或 `AbortSignal.timeout()`

### 3.2 请求取消（高优先级）

**现状**：Rust 中通过 drop oneshot receiver 实现隐式取消，Go 中**完全没有**取消机制。两端均不会向对端发送取消信号。

**gRPC 的做法**：
- 客户端可主动发送 `RST_STREAM` 取消请求
- 服务端通过 context 感知取消，停止处理
- 取消自动传播到下游调用链

**建议**：
- 新增 `CANCEL` 帧类型（或复用 TRAILERS + CANCELLED 状态码）
- Rust：通过 `CancellationToken` 向 handler 传播取消信号
- Go：通过 `context.Context` 传播取消

### 3.3 拦截器/中间件（高优先级）

**现状**：完全没有拦截器机制。用户无法在不修改 handler 逻辑的前提下添加日志、鉴权、链路追踪等横切关注点。

**gRPC 的做法**：
- 支持 Unary Interceptor 和 Stream Interceptor
- 客户端和服务端均可添加拦截器链
- 拦截器可修改请求/响应、添加 metadata、处理错误

**建议**：
```rust
// Rust 示例接口设计
type Interceptor = Box<dyn Fn(Request, Next) -> Future<Response>>;

peer.add_interceptor(|req, next| async {
    let start = Instant::now();
    let resp = next(req).await;
    println!("call took {:?}", start.elapsed());
    resp
});
```

### 3.4 Proto 服务代码生成（高优先级）

**现状**：虽然使用 protobuf 做序列化，但没有从 `.proto` 服务定义生成 RPC 存根（stub）和骨架（skeleton）代码。用户必须手动注册每个方法。

**gRPC 的做法**：
- `protoc-gen-go-grpc` / `tonic-build` 自动生成类型安全的服务接口
- 服务端只需实现 trait/interface，客户端直接调用生成的 stub

**建议**：
- 为 Rust 编写 `forpc-build` crate，基于 prost-build 扩展
- 为 Go 编写 `protoc-gen-go-forpc` 插件
- 生成类型安全的 client stub 和 server trait/interface

### 3.5 流控/背压机制（中优先级）

**现状**：使用固定大小 channel buffer（Rust 32、Go 32），无动态窗口调整，无背压反馈。高吞吐场景下可能丢帧或阻塞。

**gRPC 的做法**：
- 基于 HTTP/2 的 flow control window（初始 64KB，可调）
- 接收端通过 WINDOW_UPDATE 帧告知可接收容量
- 发送端根据窗口大小控制发送速率

**建议**：
- 引入窗口机制：发送端维护可用窗口，收到确认后滑动
- 或在 DATA 帧中加 flow_control 字段标识当前可接收容量
- 作为渐进式改进，先支持可配置的 buffer 大小

### 3.6 消息压缩（中优先级）

**现状**：完全未实现。技术规范中已定义 `:compression` 元数据键，但没有编解码实现。

**建议**：
- 支持 gzip 和 zstd 压缩算法
- 在 HEADERS 中通过 `:compression` 协商压缩方式
- 仅对超过阈值的 DATA 帧启用压缩

### 3.7 连接健康检查/Keepalive（中优先级）

**现状**：没有心跳或健康检查机制。连接断开后需要等到下一次调用失败才能发现。

**gRPC 的做法**：
- HTTP/2 PING 帧定期检查连接存活
- 服务端可配置 keepalive 参数（间隔、超时）
- 标准健康检查服务（`grpc.health.v1.Health`）

**建议**：
- 利用 stream_id=0（已保留的控制流）实现 PING/PONG
- 支持可配置的 keepalive 间隔和超时
- 提供标准健康检查 RPC 方法

### 3.8 TLS/安全传输（低优先级，视场景）

**现状**：仅支持 NNG 明文传输，没有加密能力。对于 IPC 场景可接受，TCP 场景存在安全风险。

**建议**：
- 短期：文档中明确 IPC 使用场景的安全边界
- 中期：探索 NNG TLS 支持（mangos 支持 TLS transport）
- 或在应用层实现 payload 加密

### 3.9 可观测性（低优先级）

**现状**：没有结构化日志、指标采集或分布式追踪。技术规范中已定义 `:trace-id` 元数据键但未实现。

**建议**：
- 通过拦截器机制（3.3）注入追踪逻辑
- 支持 OpenTelemetry span 上下文通过 metadata 传播
- 提供调用耗时、成功率等基础指标

### 3.10 优雅关闭（低优先级）

**现状**：支持关闭 listener 和 peer，但不保证正在进行的调用能完成。

**gRPC 的做法**：
- `GracefulStop()` 等待正在进行的调用完成后再关闭
- 支持配置最大等待时间

**建议**：
- 关闭时拒绝新调用，等待现有调用完成（带超时）
- 向对端发送 GOAWAY 信号

---

## 4. forpc 的差异化优势

在分析差距的同时，也需要认识到 forpc 相比竞品的**独特优势**：

| 优势 | 说明 |
|------|------|
| **真正的对等通信** | 连接双方均可主动发起 RPC，这是 gRPC / DRPC / Tarpc 不具备的核心能力 |
| **轻量级** | 无需 HTTP/2 协议栈，NNG 传输层开销极小 |
| **IPC 优化** | 专为本地进程间通信设计，避免网络栈开销 |
| **三语言统一协议** | Rust/Go/Node.js 使用完全相同的线路格式（wire format） |
| **简单易懂** | 协议设计清晰（3 种帧类型），容易理解和调试 |

---

## 5. 建议优先级路线图

根据**对使用者的影响**和**实现复杂度**排序：

### P0 — 基础可用性（必须优先实现）

- [ ] **超时/Deadline 执行**：调用必须有超时保护，否则一个慢调用可能阻塞整个系统
- [ ] **请求取消与传播**：调用方能取消等待中的调用，服务端能感知取消
- [ ] **拦截器/中间件框架**：没有拦截器，日志/鉴权/追踪等基础需求无法优雅实现

### P1 — 开发体验（提升框架竞争力）

- [ ] **Proto 服务代码生成**：自动生成类型安全的 client stub 和 server skeleton
- [ ] **类型安全的 Streaming API**：为流式调用提供类型化的发送/接收接口
- [ ] **更完善的错误处理**：错误链、详细错误信息（类似 gRPC RichErrorModel）

### P2 — 生产就绪（生产环境所需）

- [ ] **连接 Keepalive 与健康检查**：检测死连接，提供服务健康状态
- [ ] **流控与背压**：防止高吞吐场景下的内存溢出或消息丢失
- [ ] **消息压缩**：大消息场景下减少传输开销
- [ ] **优雅关闭**：确保关闭时正在进行的调用能正常完成

### P3 — 高级特性（长期目标）

- [ ] **TLS/安全传输**：TCP 场景下的通信加密
- [ ] **分布式追踪 (OpenTelemetry)**：与可观测性基础设施集成
- [ ] **自动重试策略**：支持指数退避的调用级重试
- [ ] **负载均衡**：多后端实例的客户端负载分配
- [ ] **浏览器/WebSocket 支持**：扩展到 Web 场景

---

## 6. 与各竞品的详细对比总结

### vs gRPC

| 维度 | forpc 优势 | gRPC 优势 |
|------|-----------|-----------|
| 通信模式 | 对等双向调用 | 仅 Client → Server |
| 传输层 | NNG 轻量，IPC 友好 | HTTP/2 功能丰富 |
| 生态 | — | 10+ 语言、海量工具、云原生集成 |
| 企业特性 | — | 拦截器、安全、可观测性、负载均衡 |

**结论**：forpc 的对等通信是核心差异化点。应优先补齐 gRPC 的基础可靠性特性（超时、取消、拦截器），而非追求 gRPC 的全部企业级功能。

### vs Cap'n Proto

| 维度 | forpc 优势 | Cap'n Proto 优势 |
|------|-----------|-----------------|
| 序列化 | protobuf 生态广泛 | 零拷贝、零解析开销 |
| 语言支持 | Node.js 一等支持 | C++ 为主 |
| 复杂度 | 协议简单 | Promise Pipelining、Capability-based Security |

**结论**：Cap'n Proto 走的是极致性能 + capability 安全方向。forpc 应保持简洁，但可以借鉴其 promise pipelining 思想优化连续调用的延迟。

### vs Tarpc

| 维度 | forpc 优势 | Tarpc 优势 |
|------|-----------|-----------|
| 跨语言 | 3 语言互通 | — |
| 通信模式 | 对等双向 + 流式 | — |
| Rust 体验 | — | 宏驱动零样板代码 |

**结论**：Tarpc 是纯 Rust 的最佳体验。forpc 在 Rust 端应借鉴其宏驱动 API 设计。

### vs vscode-jsonrpc

| 维度 | forpc 优势 | vscode-jsonrpc 优势 |
|------|-----------|-------------------|
| 序列化 | protobuf 二进制高效 | JSON 人类可读 |
| 流式 | 真正的流式传输 | 仅 Notification |
| 传输 | NNG 多协议 | stdio / WebSocket |
| 取消/进度 | — | 原生支持 |

**结论**：vscode-jsonrpc 是 IDE/工具场景的标杆。forpc 应从中借鉴取消、进度上报等对等场景的实用特性。

---

## 7. 总结

forpc 在**对等通信**和**跨语言互通**两个方面具有独特优势，但在可靠性（超时、取消）、可扩展性（拦截器、代码生成）和生产就绪（健康检查、压缩、流控）方面与主流框架有明显差距。

**核心建议**：
1. 围绕**对等 RPC** 这一差异化定位，补齐基础可靠性能力
2. 优先实现**超时、取消、拦截器**三大特性（P0）
3. 尽快提供 **Proto 服务代码生成**，大幅降低使用门槛（P1）
4. 在协议设计上保持简洁，不追求 gRPC/Cap'n Proto 的全部复杂度
