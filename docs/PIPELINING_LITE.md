# 从 Cap'n Proto 时间旅行中借鉴的轻量模式

> **日期**: 2026-03-06
> **前置文档**: [CAPNPROTO_TIME_TRAVEL.md](./CAPNPROTO_TIME_TRAVEL.md)
> **目标**: 不实现完整的 Promise Pipelining / Capability 系统，而是从中提取 **成本低、收益高** 的模式直接应用到 forpc。

---

## 核心思路

Cap'n Proto 时间旅行的本质是 **减少链式调用的网络往返次数**。完整实现需要 Promise 引用、Capability 表、Transform 导航等重型机制。但其中有几个**独立的子思想**，不需要完整的 Pipelining 基础设施就能落地，且能显著改善 forpc 的延迟和吞吐量。

| 借鉴点 | 来源 | 复杂度 | 收益 | 适合 forpc |
|--------|------|--------|------|-----------|
| 写合并（Write Coalescing） | 多个调用一次网络写入 | 低 | 减少 syscall 和小包 | ✅ |
| 批量调用（Batch RPC） | 多个独立调用打包成一组 | 中 | 减少 RTT 等待 | ✅ |
| 服务端 Handle 引用 | Capability 的极简版 | 中 | 支持有状态交互 | ✅ |
| 调用结果缓存 | Answers Table 的简化 | 低 | 幂等/去重 | ✅ |
| 客户端异步发射 | 不等结果继续发下一个 | 低 | 并发提升 | ✅ 已部分具备 |

---

## 1. 写合并（Write Coalescing）

### Cap'n Proto 的做法

Pipeline 调用天然是一组消息一起发送的——客户端把 Q1、Q2、Q3 打包在一个 write 中，省掉多次 syscall 和 TCP 小包开销。

### forpc 当前问题

Rust 和 Go 每发一个 Packet（HEADERS/DATA/TRAILERS）就做一次 `transport.send()`。一个 Unary 调用就是 3 次 send，两个并发调用就是 6 次 send——每次都是独立的 syscall + NNG 消息。

```
当前：每个 Packet 独立 send

call_1: send(HEADERS) → send(DATA) → send(TRAILERS)
call_2: send(HEADERS) → send(DATA) → send(TRAILERS)
                    ↑ 6 次 syscall
```

### 改进方案

在 transport 层增加 **write buffer**，将短时间内的多个 Packet 合并为一次网络写入：

```
改进后：合并同一 tick 的 Packet

call_1: queue(HEADERS) → queue(DATA) → queue(TRAILERS)
call_2: queue(HEADERS) → queue(DATA) → queue(TRAILERS)
                          ↓ flush
                    1 次 syscall，发送合并的 buffer
```

Node.js 端已经有 `sendQueue` 的雏形——Rust 和 Go 可以借鉴类似思路。

#### Rust 伪代码

```rust
pub struct BufferedTransport<T: Transport> {
    inner: T,
    buffer: Vec<Bytes>,
    flush_notify: tokio::sync::Notify,
}

impl<T: Transport> BufferedTransport<T> {
    pub fn queue(&self, data: Bytes) {
        self.buffer.push(data);
        self.flush_notify.notify_one();
    }

    /// 后台 task：等待一个 micro-batch 然后一次性 flush
    async fn flush_loop(&self) {
        loop {
            self.flush_notify.notified().await;
            // 等一个短暂的 micro-batch 窗口（如 50μs），收集更多消息
            tokio::time::sleep(Duration::from_micros(50)).await;
            let batch = self.buffer.drain(..);
            let merged = Self::merge_packets(batch);
            self.inner.send(merged).await;
        }
    }
}
```

#### Go 伪代码

```go
type BufferedTransport struct {
    inner    Transport
    sendCh   chan []byte
    batchUs  int // micro-batch window in microseconds
}

func (bt *BufferedTransport) Queue(data []byte) {
    bt.sendCh <- data
}

func (bt *BufferedTransport) flushLoop() {
    for {
        msg := <-bt.sendCh
        batch := [][]byte{msg}
        // 收集同一 micro-batch 窗口内的消息
        timer := time.NewTimer(time.Duration(bt.batchUs) * time.Microsecond)
    collect:
        for {
            select {
            case m := <-bt.sendCh:
                batch = append(batch, m)
            case <-timer.C:
                break collect
            }
        }
        bt.inner.Send(mergePackets(batch))
    }
}
```

**估算**：1-2 天/语言。低风险，向后兼容。

---

## 2. 批量调用（Batch RPC）

### Cap'n Proto 的做法

Pipeline 的一个副效果是多个调用在同一次网络往返中完成。即使没有依赖关系，批量发送多个独立调用也能减少等待。

### forpc 当前问题

每个 `peer.call()` / `peer.Call()` 独立等待响应。如果用户需要并发发 3 个不相关的调用，需要手动用 `tokio::join!` / `errgroup` / `Promise.all` 来并发化。

### 改进方案

提供一个 **Batch API**，将多个调用作为一组提交，框架自动并发发送并等待全部完成：

#### Rust API

```rust
let results = peer.batch(vec![
    ("User/Get",    user_id_bytes),
    ("Order/List",  order_query_bytes),
    ("Config/Get",  config_key_bytes),
]).await?;
// results: Vec<RpcResult<Bytes>>，顺序与输入一致
```

#### Go API

```go
results, err := peer.Batch([]forpc.BatchCall{
    {Method: "User/Get",    Payload: userIDBytes},
    {Method: "Order/List",  Payload: orderQueryBytes},
    {Method: "Config/Get",  Payload: configKeyBytes},
})
// results: []BatchResult，顺序与输入一致
```

#### 内部实现

Batch 不需要新的协议——只是在客户端层面将多个调用同时 fire，然后 `join` 等待：

```rust
pub async fn batch(&self, calls: Vec<(&str, Bytes)>) -> Vec<RpcResult<Bytes>> {
    let futures: Vec<_> = calls.into_iter()
        .map(|(method, payload)| self.call(method, payload))
        .collect();
    futures::future::join_all(futures).await
}
```

**关键区别**：如果结合写合并（第 1 点），这些同时发出的调用会自然被 merge 到更少的网络写入中，达到接近 Pipeline 的批量效果——但完全不需要 Promise 引用机制。

**估算**：0.5-1 天/语言。纯客户端 sugar，零协议改动。

---

## 3. 服务端 Handle 引用（轻量 Capability）

### Cap'n Proto 的做法

完整的 Capability 系统允许将"对象引用"在 Peer 之间传递，带有引用计数和生命周期管理。

### forpc 可以借鉴的简化版

不做完整的 Capability 系统，而是支持一种 **Handle 模式**：服务端在第一个调用中返回一个不透明的 handle ID，客户端在后续调用中通过 metadata 传递该 handle ID 来引用服务端的有状态资源。

```
Handle 模式（类似文件描述符）：

Client                                Server
  │                                      │
  │── Call: "FS/Open"                ──► │
  │   payload: "/home/a.txt"             │  server 创建 FileHandle, 存入 handle_map
  │◄── Response: handle_id="h42"     ── │
  │                                      │
  │── Call: "FS/Read"                ──► │
  │   metadata: {":handle": "h42"}       │  server 从 handle_map 取出 FileHandle
  │   payload: {offset: 0, len: 1024}    │
  │◄── Response: <file data>         ── │
  │                                      │
  │── Call: "FS/Close"               ──► │
  │   metadata: {":handle": "h42"}       │  server 从 handle_map 删除
  │◄── Response: ok                  ── │
```

#### 为什么不直接用 payload 传 handle？

可以，但 metadata 有几个好处：
- 拦截器/middleware 可以统一管理 handle 生命周期（如超时自动清理）
- 日志和追踪中可以看到 handle 关联
- handle 作为 metadata 是正交的，不污染业务 payload

#### 服务端 Handle 管理

```rust
/// 轻量 Handle 管理器
pub struct HandleManager {
    handles: DashMap<String, Box<dyn Any + Send + Sync>>,
    next_id: AtomicU64,
}

impl HandleManager {
    pub fn create<T: Send + Sync + 'static>(&self, resource: T) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let handle_id = format!("h{}", id);
        self.handles.insert(handle_id.clone(), Box::new(resource));
        handle_id
    }

    pub fn get<T: 'static>(&self, handle_id: &str) -> Option<&T> {
        self.handles.get(handle_id)
            .and_then(|v| v.downcast_ref::<T>())
    }

    pub fn release(&self, handle_id: &str) -> bool {
        self.handles.remove(handle_id).is_some()
    }
}
```

```go
type HandleManager struct {
    mu       sync.RWMutex
    handles  map[string]interface{}
    nextID   atomic.Uint64
}

func (hm *HandleManager) Create(resource interface{}) string {
    id := hm.nextID.Add(1)
    handleID := fmt.Sprintf("h%d", id)
    hm.mu.Lock()
    hm.handles[handleID] = resource
    hm.mu.Unlock()
    return handleID
}

func (hm *HandleManager) Get(handleID string) (interface{}, bool) {
    hm.mu.RLock()
    defer hm.mu.RUnlock()
    v, ok := hm.handles[handleID]
    return v, ok
}

func (hm *HandleManager) Release(handleID string) {
    hm.mu.Lock()
    delete(hm.handles, handleID)
    hm.mu.Unlock()
}
```

**与完整 Capability 的区别**：
- 无引用计数（显式 Close/Release）
- 不跨 Peer 传递（handle 只在创建它的 server 端有效）
- 无自动 GC（依赖客户端 Close 或 TTL 超时清理）
- 无 Transform 导航（handle 是扁平的 ID，不是嵌套引用）

**估算**：2-3 天/语言。需要定义 `:handle` metadata 规范。

---

## 4. 调用结果缓存（幂等去重）

### Cap'n Proto 的做法

Answers Table 缓存已解析的调用结果，供后续 Pipeline 调用引用。

### forpc 可以借鉴的简化版

服务端缓存最近 N 个调用结果（按 stream_id 或请求指纹），用于：

1. **幂等重试**：如果客户端因超时重发相同请求，服务端直接返回缓存结果，避免重复执行
2. **去重**：网络抖动导致的重复消息可以被识别和过滤

#### 实现方式

客户端在 metadata 中添加 `:request-id`（唯一请求 ID），服务端维护一个 **有界 LRU 缓存**：

```rust
pub struct ResultCache {
    cache: LruCache<String, CachedResult>,
}

struct CachedResult {
    response: Bytes,
    status: Status,
    cached_at: Instant,
}

impl ResultCache {
    /// 查找缓存：如果 request_id 已有结果，直接返回
    pub fn get(&mut self, request_id: &str) -> Option<&CachedResult> {
        self.cache.get(request_id)
            .filter(|c| c.cached_at.elapsed() < Duration::from_secs(60))
    }

    /// 存入缓存
    pub fn put(&mut self, request_id: String, response: Bytes, status: Status) {
        self.cache.put(request_id, CachedResult {
            response,
            status,
            cached_at: Instant::now(),
        });
    }
}
```

**重要**：只适用于 **幂等操作**。非幂等操作（如转账）不应缓存。可以通过方法名约定或 metadata 标记（`:idempotent: true`）来控制。

**估算**：1-2 天/语言。独立于其他改动。

---

## 5. 客户端异步发射（Fire-and-Collect）

### Cap'n Proto 的做法

客户端对"尚未返回的结果"发起后续调用——核心是 **不等结果就继续发下一个**。

### forpc 已有的基础

forpc 的 `alloc_stream_id` + `pending_calls` 机制天然支持多个在途调用。用户已经可以用 `tokio::spawn` / `go func()` / `Promise` 实现并发调用。

### 可以改进的地方

提供一个 **显式的 Fire-and-Collect API**，让"先发射、后收集"的模式更符合人体工程学：

```rust
// Rust：Fire-and-Collect
let ticket1 = peer.fire("User/Get", user_id)?;       // 立即返回 ticket，不等
let ticket2 = peer.fire("Order/List", query)?;        // 立即返回 ticket
let ticket3 = peer.fire("Config/Get", key)?;          // 立即返回 ticket

// ... 可以做其他事情 ...

let user = ticket1.collect().await?;     // 现在收集结果
let orders = ticket2.collect().await?;
let config = ticket3.collect().await?;
```

```go
// Go：Fire-and-Collect
t1 := peer.Fire("User/Get", userID)
t2 := peer.Fire("Order/List", query)
t3 := peer.Fire("Config/Get", key)

// ... 可以做其他事情 ...

user, err := t1.Collect()
orders, err := t2.Collect()
config, err := t3.Collect()
```

#### Ticket 实现

```rust
pub struct CallTicket {
    rx: oneshot::Receiver<RpcResult<Bytes>>,
}

impl CallTicket {
    pub async fn collect(self) -> RpcResult<Bytes> {
        self.rx.await.map_err(|_| RpcError::cancelled("call cancelled"))?
    }
}

impl RpcPeer {
    pub fn fire(&self, method: &str, payload: Bytes) -> RpcResult<CallTicket> {
        let stream_id = self.alloc_stream_id();
        let (tx, rx) = oneshot::channel();
        self.insert_pending_call(stream_id, PendingCall::new_unary(tx));

        // 发送 HEADERS + DATA + TRAILERS（不等响应）
        self.send_call_frames(stream_id, method, payload)?;

        Ok(CallTicket { rx })
    }
}
```

```go
type CallTicket struct {
    ch <-chan callResult
}

func (t *CallTicket) Collect() ([]byte, error) {
    result := <-t.ch
    return result.data, result.err
}

func (p *RpcPeer) Fire(method string, payload []byte) *CallTicket {
    streamID := p.allocStreamID()
    ch := make(chan callResult, 1)
    p.insertPendingCall(streamID, ch)
    p.sendCallFrames(streamID, method, payload)
    return &CallTicket{ch: ch}
}
```

**与 Promise Pipelining 的区别**：Fire-and-Collect 只是"并发独立调用"，不能引用前一个调用的未来结果。但结合写合并，多个 `fire()` 的网络包会自然合并，已经能覆盖大部分"减少 RTT"的收益场景。

**估算**：1-2 天/语言。纯客户端层面改动。

---

## 优先级排序

| 优先级 | 模式 | 理由 |
|--------|------|------|
| **P0** | 写合并 | 所有场景都受益，对高并发尤其明显，零协议改动 |
| **P0** | Fire-and-Collect API | 用户体验改善，自然搭配写合并 |
| **P1** | Batch API | Fire-and-Collect 的 sugar 封装，简化常见模式 |
| **P1** | 结果缓存/幂等去重 | 提高可靠性，对重试场景必要 |
| **P2** | Handle 引用 | 有状态交互需要，但先要有具体用例驱动 |

---

## 与完整 Pipelining 的对比

| | 完整 Promise Pipelining | 轻量借鉴 |
|-|----------------------|---------|
| **实现成本** | 3-6 周（四张表 + Promise 引用 + Capability 管理） | 1-2 周（五个独立小模式） |
| **协议改动** | 需要新的 metadata 键 + 可能新帧类型 | 写合并/Batch/Fire 不需要协议改动 |
| **链式调用优化** | 3 次链式 → 1 RTT | 3 次独立 → 1 RTT（如果调用不依赖前一个结果） |
| **依赖链式优化** | ✅ 有依赖也能 1 RTT | ❌ 有依赖的调用仍需串行 |
| **适用场景** | 深度链式依赖（文件系统遍历、对象图导航） | 并发独立调用、高吞吐批处理、有状态会话 |

**结论**：对于 forpc 当前的使用场景（IDE 工具间通信、微服务调用），**轻量借鉴已经覆盖 80%+ 的收益**。深度链式依赖（需要引用前一个结果）在实际使用中较少出现，可以等有了明确的需求再考虑完整 Pipelining。

---

## 参考

- [CAPNPROTO_TIME_TRAVEL.md](./CAPNPROTO_TIME_TRAVEL.md) — 完整的 Cap'n Proto Pipelining 分析
- [HTTP2_HTTP3_RESEARCH.md](./HTTP2_HTTP3_RESEARCH.md) — 写合并与 HTTP/2 multiplexing 的关联
- [GAP_ANALYSIS.md](./GAP_ANALYSIS.md) — forpc 整体差距分析和路线图
