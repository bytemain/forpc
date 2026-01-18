# forpc

forpc 是一个基于 NNG（nanomsg-next-gen）传输、使用 Apache Fory 做跨语言序列化的轻量 RPC/Streaming 框架，目标是让 Rust/Go/Node.js 之间可以用同一套协议互通。

## 特性

- 传输：NNG
  - Rust：`anng`（nng-rs）
  - Go：`mangos`（nanomsg over Go）
- 序列化：Apache Fory（默认开启 `compatible + xlang`）
- 调用模型
  - Unary（一次请求/一次响应）
  - Bidi streaming（按 stream_id 复用连接的 DATA/TRAILERS 帧流）
  - Raw 调用（不做 Fory user payload 编解码，直接传 bytes）
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
yarn run build:debug
node scripts/interop_raw_echo_server.js tcp://127.0.0.1:24002 Raw/Echo
```

另开终端：

```bash
cd rust
cargo run --example interop_raw_echo_client -- tcp://127.0.0.1:24002 Raw/Echo HelloNode
```

### Node.js 使用 JS 版 Fory（推荐）

Node 侧只负责传输字节数据，类型定义与序列化由 JS 版 Fory 完成：

```js
const { Peer, Type, createForySerializer } = require('mini-rpc')

const serializer = createForySerializer(
  Type.object('echo', {
    message: Type.string(),
  }),
)
```

## 备注

- 类型注册建议使用 `register_type_by_namespace(namespace, name)`，避免跨 crate/跨语言的 type id 冲突。
- 当前实现以“可互通/可扩展”为优先，仍在快速迭代中；协议细节以 `docs/TECHNICAL_SPECIFICATION_CN.md` 为准。
