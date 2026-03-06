# HTTP/2 与 HTTP/3 协议思路融合到 forpc 的研究

> **日期**: 2026-03-06
> **范围**: 调研 HTTP/2 和 HTTP/3 (QUIC) 协议中值得借鉴的设计思路，分析如何将这些机制融合到 forpc RPC 框架中。

---

## 目录

1. [背景：forpc 当前传输层架构](#1-背景forpc-当前传输层架构)
2. [HTTP/2 核心机制与借鉴点](#2-http2-核心机制与借鉴点)
3. [HTTP/3 (QUIC) 核心机制与借鉴点](#3-http3-quic-核心机制与借鉴点)
4. [forpc 当前差距对照](#4-forpc-当前差距对照)
5. [具体实现方案](#5-具体实现方案)
6. [分阶段实施路线图](#6-分阶段实施路线图)
7. [参考资料](#7-参考资料)

---

## 1. 背景：forpc 当前传输层架构

### 1.1 帧格式

forpc 当前的帧结构已经借鉴了 HTTP/2 的二进制分帧思想：

```
forpc 帧格式：
┌──────────────┬────────┬───────────────────┐
│ stream_id    │ kind   │ payload           │
│ (4 bytes BE) │(1 byte)│ (variable length) │
└──────────────┴────────┴───────────────────┘

3 种帧类型（类似 gRPC over HTTP/2）：
  HEADERS  (0) → protobuf 编码的 Call{method, metadata}
  DATA     (1) → 原始 payload 字节
  TRAILERS (2) → protobuf 编码的 Status{code, message}
```

### 1.2 Stream 多路复用

- 单连接承载多个并发 Stream（每个 RPC 调用一个 stream_id）
- 发起方 stream_id ≥ `0x80000000`，接收方 < `0x80000000`（奇偶分区）
- `pending_calls: HashMap<stream_id, PendingCall>` 追踪进行中的调用

### 1.3 当前的不足

| 缺失能力 | 影响 |
|----------|------|
| 无流控（Flow Control） | 快速发送方可淹没慢速接收方 |
| 无 PING/PONG | 无法检测死连接 |
| 无 RST_STREAM | 无法取消单个调用 |
| 无 GOAWAY | 无法优雅关闭连接 |
| 无 SETTINGS 协商 | 双方无法协商参数 |
| 无头部压缩 | metadata 重复传输浪费带宽 |

---

## 2. HTTP/2 核心机制与借鉴点

### 2.1 流控：WINDOW_UPDATE

**HTTP/2 原理**：

HTTP/2 的流控机制分为**连接级**和 **Stream 级**两层，防止快发送方淹没慢接收方：

```
HTTP/2 流控机制：

发送方                                    接收方
  │                                         │
  │   ◄── 初始窗口: 65535 bytes ──►         │
  │                                         │
  │──── DATA (10000 bytes) ───────────────► │  窗口剩余: 55535
  │──── DATA (20000 bytes) ───────────────► │  窗口剩余: 35535
  │──── DATA (35535 bytes) ───────────────► │  窗口剩余: 0
  │                                         │
  │     ⏸ 发送方暂停，等待窗口更新           │
  │                                         │
  │◄─── WINDOW_UPDATE(40000) ─────────────  │  接收方处理完数据
  │                                         │
  │──── DATA (40000 bytes) ───────────────► │  继续发送
```

**两层流控**：
- **连接级**：所有 Stream 共享的全局窗口（stream_id=0 的 WINDOW_UPDATE）
- **Stream 级**：每个 Stream 独立的窗口（防止单个 Stream 独占带宽）

**forpc 借鉴价值**：⭐⭐⭐⭐⭐（最高优先级）

forpc 当前仅依赖 channel buffer（256 条消息）做粗粒度背压。对于 Streaming RPC，当一端产出远快于另一端消费时，channel 满后会阻塞发送协程，但无法精细控制每个 Stream 的吞吐量。

### 2.2 连接健康：PING/PONG

**HTTP/2 原理**：

PING 帧用于连接活性检测和 RTT 测量：
- 发送方发送 PING（含 8 字节 opaque 数据）
- 接收方必须回复 PING ACK（原样返回数据）
- 超时未收到 ACK → 判定连接已死 → 关闭并重建

**gRPC 的 Keepalive 实现**：
- 每 N 秒发送 PING（默认无限制，建议 10-30s）
- 如果 M 秒内未收到 ACK，断开连接
- 配合 `KEEPALIVE_WITHOUT_CALLS` 选项控制空闲连接是否也发 PING

**forpc 借鉴价值**：⭐⭐⭐⭐⭐（最高优先级）

forpc 当前无任何活性检测。如果底层 TCP/IPC 连接因网络故障、进程崩溃等原因断开，发起方的 pending_calls 会永远阻塞等待，直到应用层超时（如果有的话）。

### 2.3 调用取消：RST_STREAM

**HTTP/2 原理**：

RST_STREAM 帧可以单方面终止一个 Stream，而不影响连接上的其他 Stream：

```
调用取消流程：

Client                                 Server
  │                                       │
  │──── HEADERS (Call: slow_query) ─────► │
  │──── DATA (params) ─────────────────► │  开始处理
  │                                       │
  │  [用户取消 / 超时]                      │
  │                                       │
  │──── RST_STREAM(CANCEL) ─────────────► │
  │                                       │  收到 RST → 停止处理
  │                                       │  释放 handler 资源
```

**gRPC 的 Cancellation 传播**：
- Client 调用 `context.cancel()` → 发送 RST_STREAM
- Server 的 `Context.Done()` channel 立即关闭 → handler 可以感知取消
- 资源（数据库连接、计算任务等）及时释放

**forpc 借鉴价值**：⭐⭐⭐⭐⭐（最高优先级）

forpc 当前缺少取消传播。GAP_ANALYSIS.md 中已识别这是 P0 优先级的差距。

### 2.4 优雅关闭：GOAWAY

**HTTP/2 原理**：

GOAWAY 帧用于通知对端"我要关闭连接了"，同时承诺会处理完已开始的请求：

```
优雅关闭流程：

Server                                 Client
  │                                       │
  │  [准备重启/维护]                        │
  │                                       │
  │──── GOAWAY(last_stream_id=7) ───────► │
  │                                       │  Stream ≤ 7：继续等待响应
  │                                       │  Stream > 7：在新连接上重试
  │                                       │
  │◄─── (继续接收 Stream 5,7 的 DATA) ──── │
  │──── 完成 Stream 5,7 的响应 ──────────► │
  │                                       │
  │  [所有 Stream 完成后关闭 TCP]            │
```

**两阶段 GOAWAY**（防止竞态）：
1. 第一次发送 `GOAWAY(last_stream_id=MAX)` → "请停止发新请求"
2. 等待一个 RTT + PING 确认
3. 第二次发送 `GOAWAY(last_stream_id=actual)` → "实际处理到这里"

**forpc 借鉴价值**：⭐⭐⭐⭐

forpc 当前关闭连接时直接断开，进行中的调用会收到错误。对于对等 RPC 场景（双方都可发起调用），优雅关闭尤为重要——关闭一端时需要等待双方的 in-flight 调用完成。

### 2.5 参数协商：SETTINGS

**HTTP/2 原理**：

连接建立时双方交换 SETTINGS 帧，协商关键参数：

| 参数 | 默认值 | 作用 |
|------|--------|------|
| `HEADER_TABLE_SIZE` | 4096 | HPACK 动态表大小 |
| `MAX_CONCURRENT_STREAMS` | 无限 | 最大并发 Stream 数 |
| `INITIAL_WINDOW_SIZE` | 65535 | 初始流控窗口 |
| `MAX_FRAME_SIZE` | 16384 | 最大帧大小 |
| `MAX_HEADER_LIST_SIZE` | 无限 | 最大 header 总大小 |

**forpc 借鉴价值**：⭐⭐⭐

可以在连接握手阶段协商：最大并发调用数、流控窗口大小、是否启用压缩、协议版本等。

### 2.6 头部压缩：HPACK

**HTTP/2 原理**：

HPACK 使用静态表（61 个预定义 header）+ 动态表（最近发送过的 header）+ Huffman 编码，对重复的 header 做增量压缩：

```
首次发送：
  method: "UserService/GetUser"     →  完整编码 (30 bytes)
  :timeout: "5000"                  →  完整编码 (12 bytes)
  authorization: "Bearer token123"  →  完整编码 (25 bytes)
                                       总计: 67 bytes

第二次发送（相同 header）：
  method: "UserService/GetUser"     →  索引引用 (1 byte)
  :timeout: "5000"                  →  索引引用 (1 byte)
  authorization: "Bearer token123"  →  索引引用 (1 byte)
                                       总计: 3 bytes（压缩率 95%）
```

**forpc 借鉴价值**：⭐⭐

forpc 的 metadata 是 `map<string, string>`，对于高频调用（如同一方法名反复调用），metadata 会重复传输。但 forpc 主要用于 IPC 场景，带宽通常不是瓶颈，优先级较低。

### 2.7 Stream 优先级

**HTTP/2 原理**：

HTTP/2 允许客户端为 Stream 分配权重和依赖关系，引导服务端优先处理关键请求。

**forpc 借鉴价值**：⭐⭐

RPC 场景中可以区分 health check（高优先级）与普通调用（正常优先级）、批量数据传输（低优先级）。

---

## 3. HTTP/3 (QUIC) 核心机制与借鉴点

### 3.1 独立的 Stream 丢包恢复（消除队头阻塞）

**HTTP/2 over TCP 的问题**：

```
HTTP/2 over TCP 的队头阻塞：

Stream A: [pkt1] [pkt2] [pkt3]
Stream B: [pkt4] [pkt5] [pkt6]
Stream C: [pkt7] [pkt8] [pkt9]

         ──── TCP 字节流 ────
         pkt1 pkt4 pkt7 pkt2 pkt5 pkt8 pkt3 pkt6 pkt9

如果 pkt4 丢失：
  ✅ pkt1 已到达 → Stream A 可以处理
  ❌ pkt4 丢失  → Stream B 等待重传
  ❌ pkt7 已到达 → Stream C 也被阻塞！（TCP 按序交付）

  TCP 层面：pkt7 虽然已到达，但因为 pkt4 还没到，
  TCP 不会将 pkt7 交付给应用层。所有 Stream 都被卡住。
```

**QUIC 的解决方案**：

```
QUIC 的独立 Stream 丢包恢复：

如果 pkt4 丢失：
  ✅ pkt1 已到达 → Stream A 立即处理
  ❌ pkt4 丢失  → 仅 Stream B 等待重传
  ✅ pkt7 已到达 → Stream C 立即处理！

  QUIC 层面：每个 Stream 独立排序，
  一个 Stream 的丢包不影响其他 Stream。
```

**forpc 借鉴价值**：⭐⭐⭐⭐

forpc 当前跑在 TCP 上（NNG over TCP）。如果两个独立的 RPC 调用共享一个 TCP 连接，其中一个调用的数据包丢失会阻塞所有调用。切换到 QUIC 传输层可以彻底消除这个问题。

### 3.2 0-RTT 连接恢复

**传统 TCP + TLS 的握手延迟**：

```
TCP + TLS 1.2：需要 3 个 RTT 才能开始发数据

Client                                 Server
  │──── SYN ───────────────────────────►│  RTT 1 (TCP)
  │◄─── SYN-ACK ──────────────────────│
  │──── ACK ───────────────────────────►│
  │                                     │
  │──── ClientHello ───────────────────►│  RTT 2 (TLS)
  │◄─── ServerHello + Certificate ─────│
  │──── Finished ──────────────────────►│  RTT 3 (TLS)
  │◄─── Finished ──────────────────────│
  │                                     │
  │──── 开始发送 RPC 数据 ──────────────►│  终于可以发数据了！
```

**QUIC 的 0-RTT 恢复**：

```
QUIC 0-RTT（恢复连接）：第一个 packet 就带数据！

Client                                 Server
  │                                     │
  │──── Initial + 0-RTT Data ──────────►│  立即发送 RPC 请求！
  │◄─── Handshake + Response ──────────│
  │                                     │

  总延迟: 0 RTT（数据随第一个包一起发送）
```

**注意**：0-RTT 数据可能被重放攻击，只适合幂等操作。

**forpc 借鉴价值**：⭐⭐⭐

对于需要频繁断开/重连的场景（如移动设备、网络切换），0-RTT 能显著减少首次调用延迟。但 forpc 主要面向 IPC 场景（本机通信），连接建立延迟通常很小，优先级中等。

### 3.3 连接迁移（Connection Migration）

**TCP 的问题**：TCP 连接绑定到 (src_ip, src_port, dst_ip, dst_port) 四元组。任何一个变化（如 WiFi → 4G 切换），连接必须重建。

**QUIC 的解决方案**：使用 Connection ID（与 IP 无关）标识连接。网络切换时，新 IP 发送的 QUIC 包可以被关联到已有连接，无需重建。

```
QUIC 连接迁移：

Client (WiFi: 192.168.1.10)            Server
  │──── ConnID=ABC, RPC Request ──────► │
  │                                      │
  │  [WiFi → 4G 切换]                    │
  │  IP 变为 10.0.0.5                    │
  │                                      │
  │──── ConnID=ABC, RPC Request ──────► │  同一个 Connection ID！
  │◄─── Response ──────────────────────│  无缝继续
```

**forpc 借鉴价值**：⭐⭐

对 IPC 场景价值有限（本机通信不涉及网络切换），但对跨网络的 TCP 传输模式有一定意义。

### 3.4 内置 TLS 1.3

**QUIC 的特点**：QUIC 将 TLS 1.3 握手合并到传输握手中，加密是协议的内置组成部分而非可选层。

**forpc 借鉴价值**：⭐⭐⭐

GAP_ANALYSIS.md 已识别 TLS 为 P3 优先级。如果 forpc 添加 QUIC 传输支持，TLS 会自动获得。

### 3.5 QPACK 头部压缩

**与 HPACK 的区别**：HPACK 要求按序编解码，在 QUIC 多 Stream 并行场景下会造成队头阻塞。QPACK 改为异步更新动态表，避免了这个问题。

**forpc 借鉴价值**：⭐⭐

如果 forpc 引入头部压缩并同时支持 QUIC，应优先考虑 QPACK 而非 HPACK。

---

## 4. forpc 当前差距对照

将 HTTP/2 和 HTTP/3 的关键机制与 forpc 现状做完整对照：

| 机制 | HTTP/2 | HTTP/3 (QUIC) | forpc 现状 | 优先级 | 借鉴方案 |
|------|--------|---------------|-----------|--------|---------|
| **流控** | WINDOW_UPDATE | QUIC 内置 | ❌ 仅 channel buffer | P0 | 实现双层窗口流控 |
| **活性检测** | PING/PONG | QUIC 内置 | ❌ 无 | P0 | 新增 PING/PONG 帧 |
| **调用取消** | RST_STREAM | STOP_SENDING | ❌ 无法传播取消 | P0 | 新增 RST_STREAM 帧 |
| **优雅关闭** | GOAWAY | QUIC 内置 | ❌ 直接断开 | P1 | 新增 GOAWAY 帧 |
| **参数协商** | SETTINGS | Transport Params | ❌ 无 | P1 | 连接握手时交换 SETTINGS |
| **头部压缩** | HPACK | QPACK | ❌ 明文传输 | P2 | 实现简化版 HPACK |
| **Stream 优先级** | PRIORITY | PRIORITY_UPDATE | ❌ 无 | P2 | metadata 中携带优先级 |
| **队头阻塞消除** | ❌ (TCP 固有) | ✅ 独立 Stream | ❌ (TCP 固有) | P3 | 添加 QUIC 传输后端 |
| **0-RTT 连接** | ❌ | ✅ | ❌ | P3 | QUIC 传输层自带 |
| **连接迁移** | ❌ | ✅ | ❌ | P3 | QUIC 传输层自带 |
| **内置加密** | ❌ (TLS 可选) | ✅ (TLS 1.3 必选) | ❌ | P3 | QUIC 传输层自带 |

---

## 5. 具体实现方案

### 5.1 新增帧类型

当前 forpc 有 3 种帧类型（HEADERS=0, DATA=1, TRAILERS=2）。借鉴 HTTP/2，建议新增以下控制帧：

```
新增帧类型：

  ┌─────┬────────────────┬─────────────────────────────────┐
  │ Kind│ 名称           │ Payload                          │
  ├─────┼────────────────┼─────────────────────────────────┤
  │  0  │ HEADERS        │ protobuf Call (现有)              │
  │  1  │ DATA           │ 原始 bytes (现有)                │
  │  2  │ TRAILERS       │ protobuf Status (现有)           │
  │  3  │ RST_STREAM     │ error_code: u32                  │
  │  4  │ SETTINGS       │ key-value pairs                  │
  │  5  │ PING           │ opaque: u64                      │
  │  6  │ GOAWAY         │ last_stream_id: u32 + reason: u32│
  │  7  │ WINDOW_UPDATE  │ increment: u32                   │
  └─────┴────────────────┴─────────────────────────────────┘

  控制帧使用 stream_id=0（连接级）
  RST_STREAM 和 WINDOW_UPDATE 使用实际 stream_id（Stream 级）
```

### 5.2 流控实现

#### 5.2.1 数据结构

```rust
// ============= Rust 实现 =============

/// 流控窗口
pub struct FlowControlWindow {
    /// 当前可发送的字节数
    available: AtomicI64,
    /// 初始窗口大小（用于重置）
    initial_size: u32,
    /// 当窗口为 0 时，等待 WINDOW_UPDATE 的通知
    notify: tokio::sync::Notify,
}

impl FlowControlWindow {
    pub fn new(initial_size: u32) -> Self {
        Self {
            available: AtomicI64::new(initial_size as i64),
            initial_size,
            notify: tokio::sync::Notify::new(),
        }
    }

    /// 尝试消耗窗口，如果窗口不足则等待
    pub async fn acquire(&self, bytes: u32) {
        loop {
            let current = self.available.load(Ordering::Relaxed);
            if current >= bytes as i64 {
                if self.available.compare_exchange(
                    current,
                    current - bytes as i64,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    return;
                }
            } else {
                // 窗口不足，等待 WINDOW_UPDATE
                self.notify.notified().await;
            }
        }
    }

    /// 收到 WINDOW_UPDATE 后增加窗口
    pub fn release(&self, increment: u32) {
        self.available.fetch_add(increment as i64, Ordering::Release);
        self.notify.notify_waiters();
    }
}

/// Peer 级别的流控管理
pub struct FlowController {
    /// 连接级窗口（所有 Stream 共享）
    connection_window: FlowControlWindow,
    /// Stream 级窗口
    stream_windows: DashMap<u32, FlowControlWindow>,
}
```

```go
// ============= Go 实现 =============

// FlowControlWindow 管理单个窗口
type FlowControlWindow struct {
    mu        sync.Mutex
    available int64
    initial   uint32
    notify    chan struct{} // 窗口更新通知
}

func NewFlowControlWindow(initial uint32) *FlowControlWindow {
    return &FlowControlWindow{
        available: int64(initial),
        initial:   initial,
        notify:    make(chan struct{}, 1),
    }
}

// Acquire 获取发送配额，如果窗口不足则阻塞等待
func (w *FlowControlWindow) Acquire(ctx context.Context, bytes uint32) error {
    for {
        w.mu.Lock()
        if w.available >= int64(bytes) {
            w.available -= int64(bytes)
            w.mu.Unlock()
            return nil
        }
        w.mu.Unlock()

        select {
        case <-ctx.Done():
            return ctx.Err()
        case <-w.notify:
            // 窗口已更新，重试
        }
    }
}

// Release 收到 WINDOW_UPDATE 后增加窗口
func (w *FlowControlWindow) Release(increment uint32) {
    w.mu.Lock()
    w.available += int64(increment)
    w.mu.Unlock()

    select {
    case w.notify <- struct{}{}:
    default:
    }
}
```

#### 5.2.2 发送端流控逻辑

```rust
// 发送 DATA 帧前的流控检查
async fn send_data_with_flow_control(
    &self,
    stream_id: u32,
    data: Bytes,
) -> Result<()> {
    let data_len = data.len() as u32;

    // 1. 等待连接级窗口
    self.flow_controller.connection_window.acquire(data_len).await;

    // 2. 等待 Stream 级窗口
    if let Some(window) = self.flow_controller.stream_windows.get(&stream_id) {
        window.acquire(data_len).await;
    }

    // 3. 窗口足够，发送数据
    self.send_frame(stream_id, FrameKind::Data, data).await
}
```

#### 5.2.3 接收端窗口更新策略

```rust
// 接收端处理 DATA 帧后更新窗口
fn on_data_received(&self, stream_id: u32, data_len: u32) {
    self.consumed_bytes.fetch_add(data_len as u64, Ordering::Relaxed);

    let consumed = self.consumed_bytes.load(Ordering::Relaxed);
    let threshold = self.initial_window_size as u64 / 2;

    // 当消耗超过窗口的一半时，发送 WINDOW_UPDATE
    if consumed >= threshold {
        self.consumed_bytes.fetch_sub(consumed, Ordering::Relaxed);

        // 发送连接级 WINDOW_UPDATE
        self.send_window_update(0, consumed as u32);
        // 发送 Stream 级 WINDOW_UPDATE
        self.send_window_update(stream_id, consumed as u32);
    }
}
```

### 5.3 PING/PONG 实现

```rust
// ============= Rust 实现 =============

pub struct KeepaliveManager {
    /// PING 间隔
    interval: Duration,
    /// PING 超时
    timeout: Duration,
    /// 上次收到 PONG 的时间
    last_pong: Instant,
}

impl KeepaliveManager {
    /// 后台任务：定期发送 PING
    pub async fn run(&self, peer: Arc<RpcPeer>) {
        let mut interval = tokio::time::interval(self.interval);
        loop {
            interval.tick().await;

            let ping_data = rand::random::<u64>();
            peer.send_frame(0, FrameKind::Ping, ping_data.to_be_bytes()).await;

            // 等待 PONG
            tokio::time::sleep(self.timeout).await;
            if self.last_pong.elapsed() > self.interval + self.timeout {
                // 连接已死
                peer.close_with_error("keepalive timeout").await;
                return;
            }
        }
    }

    /// 收到 PING 时回复 PONG
    pub async fn on_ping(&self, peer: &RpcPeer, data: u64) {
        // PONG = PING 帧 + 原样返回数据（通过 metadata 区分）
        peer.send_frame(0, FrameKind::Ping, data.to_be_bytes()).await;
    }

    /// 收到 PONG 时更新时间
    pub fn on_pong(&mut self) {
        self.last_pong = Instant::now();
    }
}
```

### 5.4 RST_STREAM（调用取消）实现

```rust
// ============= Rust 实现 =============

/// 取消一个进行中的调用
pub async fn cancel_call(&self, stream_id: u32) -> Result<()> {
    // 1. 发送 RST_STREAM 给对端
    let error_code: u32 = 8; // CANCEL（与 HTTP/2 一致）
    self.send_frame(stream_id, FrameKind::RstStream, error_code.to_be_bytes()).await?;

    // 2. 清理本地 pending_call
    if let Some(pending) = self.pending_calls.remove(&stream_id) {
        let _ = pending.tx.send(Err(RpcError::cancelled("call cancelled by client")));
    }

    Ok(())
}

/// 收到 RST_STREAM 时的处理
fn on_rst_stream(&self, stream_id: u32, error_code: u32) {
    // 1. 如果是我们发起的调用 → 通知等待方
    if let Some(pending) = self.pending_calls.remove(&stream_id) {
        let _ = pending.tx.send(Err(RpcError::cancelled("call cancelled by remote")));
    }

    // 2. 如果是对端发起的调用 → 通知 handler 取消
    if let Some(inbound) = self.inbound_streams.remove(&stream_id) {
        // 关闭 handler 的 packet channel → handler 感知取消
        drop(inbound.tx);
    }
}
```

```go
// ============= Go 实现 =============

// CancelCall 取消一个进行中的调用
func (p *RpcPeer) CancelCall(streamID uint32) error {
    // 1. 发送 RST_STREAM
    errorCode := uint32(8) // CANCEL
    buf := make([]byte, 4)
    binary.BigEndian.PutUint32(buf, errorCode)
    if err := p.sendFrame(streamID, FrameKindRstStream, buf); err != nil {
        return err
    }

    // 2. 清理本地 pending call
    p.mu.Lock()
    if pc, ok := p.pending[streamID]; ok {
        delete(p.pending, streamID)
        p.mu.Unlock()
        pc.unaryCh <- resultBytes{err: ErrCancelled}
    } else {
        p.mu.Unlock()
    }

    return nil
}
```

### 5.5 GOAWAY（优雅关闭）实现

```rust
// ============= Rust 实现 =============

/// 优雅关闭连接
pub async fn graceful_shutdown(&self, timeout: Duration) -> Result<()> {
    // Phase 1: 通知对端停止发送新请求
    let last_stream_id = self.max_received_stream_id.load(Ordering::Relaxed);
    self.send_goaway(last_stream_id, 0 /* NO_ERROR */).await?;

    // Phase 2: 等待所有 in-flight 调用完成（或超时）
    let deadline = Instant::now() + timeout;
    loop {
        if self.pending_calls.is_empty() && self.inbound_streams.is_empty() {
            break; // 所有调用已完成
        }
        if Instant::now() > deadline {
            break; // 超时，强制关闭
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Phase 3: 关闭连接
    self.close().await
}

/// 收到 GOAWAY 时的处理
fn on_goaway(&self, last_stream_id: u32, error_code: u32) {
    self.is_goaway_received.store(true, Ordering::Release);

    // 对于 stream_id > last_stream_id 的 pending calls，标记需要重试
    for entry in self.pending_calls.iter() {
        if *entry.key() > last_stream_id {
            let _ = entry.value().tx.send(
                Err(RpcError::unavailable("connection going away, retry on new connection"))
            );
        }
    }
}
```

### 5.6 SETTINGS（参数协商）实现

```rust
// ============= Rust 实现 =============

/// 连接参数
#[derive(Debug, Clone)]
pub struct ConnectionSettings {
    /// 最大并发 Stream 数
    pub max_concurrent_streams: u32,     // 默认: 100
    /// 初始流控窗口大小
    pub initial_window_size: u32,        // 默认: 65535
    /// 最大帧大小
    pub max_frame_size: u32,             // 默认: 16384
    /// 是否启用 Keepalive
    pub keepalive_enabled: bool,         // 默认: true
    /// Keepalive 间隔（毫秒）
    pub keepalive_interval_ms: u32,      // 默认: 30000
    /// 协议版本
    pub protocol_version: u32,           // 默认: 1
}

/// 连接握手：交换 SETTINGS
pub async fn handshake(&self) -> Result<ConnectionSettings> {
    // 1. 发送本端 SETTINGS
    let local = self.local_settings.encode();
    self.send_frame(0, FrameKind::Settings, local).await?;

    // 2. 等待对端 SETTINGS
    let remote_frame = self.recv_settings().await?;
    let remote = ConnectionSettings::decode(&remote_frame.payload)?;

    // 3. 取双方的最小值作为生效参数
    let effective = ConnectionSettings {
        max_concurrent_streams: local.max_concurrent_streams
            .min(remote.max_concurrent_streams),
        initial_window_size: local.initial_window_size
            .min(remote.initial_window_size),
        // ...
    };

    // 4. 发送 SETTINGS ACK
    self.send_frame(0, FrameKind::Settings, &[/* ACK flag */]).await?;

    Ok(effective)
}
```

### 5.7 QUIC 传输后端

如果 forpc 要获得 HTTP/3 级别的能力（消除队头阻塞、0-RTT、连接迁移、内置 TLS），最直接的方式是添加 QUIC 传输后端：

```rust
// ============= Rust：基于 quinn 的 QUIC 传输 =============

use quinn::{Endpoint, Connection, RecvStream, SendStream};

pub struct QuicTransport {
    connection: Connection,
}

impl QuicTransport {
    /// 连接到远端
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
        let connection = endpoint.connect(addr, "forpc")?.await?;
        Ok(Self { connection })
    }

    /// 发送帧 → 利用 QUIC 原生 Stream
    pub async fn send_frame(
        &self,
        stream_id: u32,
        kind: u8,
        payload: &[u8],
    ) -> Result<()> {
        // 每个 RPC 调用使用一个 QUIC 双向 Stream
        // QUIC Stream 天然具备：
        // - 独立丢包恢复（无队头阻塞）
        // - 内置流控
        // - 内置取消（STOP_SENDING / RESET_STREAM）
        let (mut send, _recv) = self.connection.open_bi().await?;
        let header = encode_frame_header(stream_id, kind, payload.len());
        send.write_all(&header).await?;
        send.write_all(payload).await?;
        Ok(())
    }
}
```

```go
// ============= Go：基于 quic-go 的 QUIC 传输 =============

import "github.com/quic-go/quic-go"

type QuicTransport struct {
    conn quic.Connection
}

func (t *QuicTransport) Connect(addr string) error {
    conn, err := quic.DialAddr(context.Background(), addr, &tls.Config{
        InsecureSkipVerify: true, // 开发环境
        NextProtos:         []string{"forpc"},
    }, nil)
    if err != nil {
        return err
    }
    t.conn = conn
    return nil
}

func (t *QuicTransport) SendFrame(streamID uint32, kind byte, payload []byte) error {
    // 每个 RPC 使用一个 QUIC Stream
    stream, err := t.conn.OpenStreamSync(context.Background())
    if err != nil {
        return err
    }
    header := encodeFrameHeader(streamID, kind, len(payload))
    stream.Write(header)
    stream.Write(payload)
    return nil
}
```

**QUIC 传输层带来的免费能力**：

| 能力 | 说明 |
|------|------|
| ✅ 无队头阻塞 | 每个 QUIC Stream 独立恢复 |
| ✅ 内置流控 | QUIC 自带 Stream 级和连接级流控 |
| ✅ 内置取消 | STOP_SENDING / RESET_STREAM |
| ✅ 0-RTT 恢复 | 断开后快速重连 |
| ✅ 连接迁移 | IP 变化不影响连接 |
| ✅ TLS 1.3 | 加密内置到协议中 |
| ✅ 拥塞控制 | 用户态实现，可定制算法 |

---

## 6. 分阶段实施路线图

### Phase 0：核心控制帧（P0）

**目标**：实现 HTTP/2 中对 RPC 最关键的三个控制机制

**工作内容**：
- [ ] 扩展帧类型：新增 RST_STREAM (3), PING (5), WINDOW_UPDATE (7)
- [ ] 实现 PING/PONG keepalive（Rust / Go / Node）
- [ ] 实现 RST_STREAM 调用取消 + handler 端取消感知
- [ ] 实现双层流控（连接级 + Stream 级 WINDOW_UPDATE）
- [ ] 单元测试 + 跨语言互通测试

**估算**：Rust 5-7 天，Go 3-5 天，Node 2-3 天

### Phase 1：连接管理（P1）

**目标**：实现可靠的连接生命周期管理

**工作内容**：
- [ ] 新增 SETTINGS 帧 (4) + 连接握手协商
- [ ] 新增 GOAWAY 帧 (6) + 两阶段优雅关闭
- [ ] 并发限制：基于 `max_concurrent_streams` 拒绝超额请求
- [ ] 连接状态机：OPEN → GOAWAY_SENT → DRAINING → CLOSED

**估算**：Rust 3-5 天，Go 2-3 天，Node 2 天

### Phase 2：性能优化（P2）

**目标**：借鉴 HTTP/2 的性能优化手段

**工作内容**：
- [ ] 简化版 HPACK：为 method 和常用 metadata key 建立静态/动态表
- [ ] Stream 优先级：metadata 中支持 `:priority` 字段
- [ ] 帧合并（Frame Coalescing）：小帧合并为大帧减少系统调用

**估算**：3-5 天

### Phase 3：QUIC 传输后端（P3）

**目标**：添加 QUIC 作为可选传输层，一步获得 HTTP/3 级别能力

**工作内容**：
- [ ] Rust：基于 `quinn` 实现 QUIC 传输后端
- [ ] Go：基于 `quic-go` 实现 QUIC 传输后端
- [ ] Node：基于原生 QUIC 或 ngtcp2 绑定
- [ ] 当使用 QUIC 时，RST_STREAM/WINDOW_UPDATE 退化为 QUIC 原生机制
- [ ] 传输层抽象：统一 NNG(TCP/IPC) 和 QUIC 的接口
- [ ] 性能对比测试：NNG vs QUIC（延迟、吞吐量、丢包场景）

**估算**：Rust 7-10 天，Go 5-7 天，Node 5 天

---

## 7. 参考资料

- [RFC 9113: HTTP/2](https://www.rfc-editor.org/rfc/rfc9113.html) — HTTP/2 协议规范（流控、帧类型、HPACK 等）
- [RFC 9114: HTTP/3](https://www.rfc-editor.org/rfc/rfc9114.html) — HTTP/3 协议规范
- [RFC 9000: QUIC](https://www.ietf.org/rfc/rfc9000.html) — QUIC 传输协议规范
- [gRPC over HTTP/2 协议映射](https://github.com/grpc/grpc/blob/master/doc/PROTOCOL-HTTP2.md) — gRPC 如何使用 HTTP/2 帧
- [gRPC on HTTP/2: Engineering a Robust Protocol](https://grpc.io/blog/grpc-on-http2/) — gRPC 团队的 HTTP/2 工程实践
- [quinn](https://github.com/quinn-rs/quinn) — Rust 的 QUIC 实现
- [quic-go](https://github.com/quic-go/quic-go) — Go 的 QUIC 实现
