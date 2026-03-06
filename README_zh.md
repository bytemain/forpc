# forpc

forpc 是一个基于 NNG（nanomsg-next-gen）传输、使用 Protocol Buffers（protobuf）做跨语言序列化的轻量 RPC/Streaming 框架，目标是让 Rust/Go/Node.js 之间可以用同一套协议互通。

## 特性

- 传输：NNG
  - Rust：`anng`（nng-rs）
  - Go：`mangos`（nanomsg over Go）
- 序列化：默认 Protocol Buffers（protobuf），支持通过 Raw 调用使用自定义序列化
  - Rust：`prost` crate（使用 derive 宏）
  - Go：`google.golang.org/protobuf`
  - 自定义：任何序列化格式（MessagePack、JSON、FlatBuffers 等）通过 `callRaw` / `CallRaw` 实现
- 调用模型
  - Unary（一次请求/一次响应）
  - Bidi streaming（按 stream_id 复用连接的 DATA/TRAILERS 帧流）
  - Raw 调用（不做 protobuf user payload 编解码，直接传 bytes — 可用于自定义序列化）
- 跨语言互通：Rust ↔ Go ↔ Node.js（Node 通过 napi-rs 绑定 Rust 实现）

## 目录结构

- `rust/`：Rust 实现（crate: `forpc`）
- `go/`：Go 实现（module: `github.com/bytemain/forpc/go`）
- `node/`：Node.js 原生扩展（napi-rs，绑定 Rust `forpc`）
- `scripts/interop_matrix.sh`：互通矩阵测试（client/server 笛卡尔积）
- `docs/TECHNICAL_SPECIFICATION_CN.md`：协议与实现细节

## 快速验证（推荐）

直接跑互通矩阵：

```bash
bash scripts/interop_matrix.sh
```

该脚本会构建 Rust/Go examples、构建 Node addon，并跑通：

- Rust server → Go client
- Go server → Rust client
- Go server → Go client
- Rust server → Rust client
- Rust raw server → Node raw client
- Node raw server → Rust raw client
- Go raw server → Node raw client
- Node raw server → Go raw client
- Node raw server → Node raw client
- Rust msgpack server → Go msgpack client（跨语言 MessagePack）
- Go msgpack server → Rust msgpack client
- Rust msgpack server → Node msgpack client
- Node msgpack server → Rust msgpack client
- Go msgpack server → Node msgpack client
- Node msgpack server → Go msgpack client

## 手动运行示例

### Rust server → Go client（Unary）

```bash
cd rust
cargo run --example interop_echo_server -- tcp://127.0.0.1:24000
```

另开终端：

```bash
cd go
go run ./examples/interop_echo_client --connect tcp://127.0.0.1:24000 --msg Hello
```

### Node server（JS handler）→ Rust client（Raw）

```bash
cd node
npm run build:debug
node scripts/interop_raw_echo_server.js tcp://127.0.0.1:24002 Raw/Echo
```

另开终端：

```bash
cd rust
cargo run --example interop_raw_echo_client -- tcp://127.0.0.1:24002 Raw/Echo HelloNode
```

## 自定义序列化（MessagePack 示例）

forpc 默认使用 Protocol Buffers 作为类型化 `call()` API 的序列化格式，同时提供 `callRaw()` / `CallRaw()` / `call_raw()` 接口，直接传输原始字节，不做任何框架层面的序列化。这使得你可以轻松使用**任何**序列化格式 — MessagePack、JSON、FlatBuffers、Cap'n Proto 或自定义二进制协议 — 同时享受 forpc 的传输、帧分组、多路复用和跨语言互通能力。

### 工作原理

```
┌──────────┐     callRaw(method, bytes)     ┌──────────┐
│  客户端   │ ────────────────────────────►  │  服务端   │
│           │                                │           │
│ encode()  │   forpc 负责帧分组、           │ decode()  │
│ decode()  │   路由、多路复用               │ encode()  │
│           │  ◄──────────────────────────── │           │
└──────────┘     原始字节响应                └──────────┘
```

应用层负责序列化/反序列化 payload；forpc 处理其余一切（传输、流多路复用、元数据、超时、错误传播）。

### MessagePack 库

| 语言 | 库 | 安装 |
|------|-----|------|
| Rust | [`rmp-serde`](https://crates.io/crates/rmp-serde) | `cargo add rmp-serde` |
| Go | [`github.com/vmihailenco/msgpack/v5`](https://github.com/vmihailenco/msgpack) | `go get github.com/vmihailenco/msgpack/v5` |
| Node.js | [`@msgpack/msgpack`](https://www.npmjs.com/package/@msgpack/msgpack) | `npm install @msgpack/msgpack` |

### Rust 示例

```rust
use bytes::Bytes;
use forpc::RpcPeer;

#[derive(serde::Serialize, serde::Deserialize)]
struct EchoMsg { message: String }

// 客户端：使用 MessagePack 编码，通过 call_raw 发送
let req = EchoMsg { message: "Hello".into() };
let payload = rmp_serde::to_vec_named(&req)?;  // 使用 to_vec_named 以保持跨语言兼容
let resp = peer.call_raw("MsgPack/Echo", Bytes::from(payload)).await?;
let reply: EchoMsg = rmp_serde::from_slice(&resp)?;
println!("reply: {}", reply.message);
```

### Go 示例

```go
import "github.com/vmihailenco/msgpack/v5"

type EchoMsg struct {
    Message string `msgpack:"message"`
}

// 客户端：使用 MessagePack 编码，通过 CallRaw 发送
payload, _ := msgpack.Marshal(&EchoMsg{Message: "Hello"})
resp, _ := peer.CallRaw("MsgPack/Echo", payload)
var reply EchoMsg
msgpack.Unmarshal(resp, &reply)
fmt.Println("reply:", reply.Message)
```

### Node.js 示例

```js
const { encode, decode } = require('@msgpack/msgpack')

// 客户端：使用 MessagePack 编码，通过 callRaw 发送
const resp = await peer.callRaw('MsgPack/Echo', Buffer.from(encode({ message: 'Hello' })))
const reply = decode(resp)
console.log('reply:', reply.message)
```

### 运行 MessagePack 示例

启动一种语言的服务端，用另一种语言的客户端连接 — MessagePack 是跨语言的二进制格式，所有组合都能互通：

```bash
# Go 服务端
cd go && go run ./examples/msgpack_echo_server --listen tcp://127.0.0.1:24010

# Rust 客户端（另开终端）
cd rust && cargo run --example msgpack_echo_client -- tcp://127.0.0.1:24010 MsgPack/Echo "Hello from Rust"
```

```bash
# Rust 服务端
cd rust && cargo run --example msgpack_echo_server -- tcp://127.0.0.1:24010

# Node.js 客户端（另开终端）
cd node && node scripts/msgpack_echo_client.js tcp://127.0.0.1:24010 MsgPack/Echo "Hello from Node"
```

完整示例代码位于各语言目录中：
- Rust：`rust/examples/msgpack_echo_server.rs`、`rust/examples/msgpack_echo_client.rs`
- Go：`go/examples/msgpack_echo_server/`、`go/examples/msgpack_echo_client/`
- Node.js：`node/scripts/msgpack_echo_server.js`、`node/scripts/msgpack_echo_client.js`

### 使用其他序列化格式

同样的 `callRaw` 模式适用于任何序列化格式，只需替换编解码调用：

| 格式 | Rust | Go | Node.js |
|------|------|-----|---------|
| **MessagePack** | `rmp-serde` | `vmihailenco/msgpack/v5` | `@msgpack/msgpack` |
| **JSON** | `serde_json` | `encoding/json` | `JSON.stringify/parse` |
| **FlatBuffers** | `flatbuffers` | `google/flatbuffers` | `flatbuffers` |
| **Cap'n Proto** | `capnp` | `capnproto/go-capnp` | `capnp-ts` |
| **CBOR** | `ciborium` | `fxamacker/cbor/v2` | `cbor-x` |
| **自定义二进制** | 手动 `bytes` | 手动 `[]byte` | 手动 `Buffer` |

## 重新生成 Protobuf 代码

生成的 Rust protobuf 代码已预先提交在 `rust/src/gen/` 目录中，正常构建不需要 `protoc`。当 `.proto` 文件变更后，运行以下命令重新生成：

```bash
cd rust
FORPC_GENERATE_PROTO=1 cargo build
```

需要安装 `protoc`，详见 [protobuf 安装说明](https://github.com/protocolbuffers/protobuf#protobuf-compiler-installation)。

## 备注

- 当前实现以“可互通/可扩展”为优先，仍在快速迭代中；协议细节以 `docs/TECHNICAL_SPECIFICATION_CN.md` 为准。

