# forpc
<p align="center">
  <a href="./README_zh.md">简体中文</a>
</p>

forpc is a lightweight RPC/Streaming framework based on NNG (nanomsg-next-gen) transport, using Protocol Buffers (protobuf) for cross-language serialization. Its goal is to enable seamless communication between Rust/Go/Node.js using a unified protocol.

## Features

- Transport: NNG
  - Rust: `anng` (nng-rs)
  - Go: `mangos` (nanomsg over Go)
- Serialization: Protocol Buffers (protobuf) by default, with support for custom codecs via raw calls
  - Rust: `prost` crate with derive macros
  - Go: `google.golang.org/protobuf`
  - Custom: Any serialization format (MessagePack, JSON, FlatBuffers, etc.) via `callRaw` / `CallRaw`
- Call Models
  - Unary (single request/single response)
  - Bidi streaming (DATA/TRAILERS frame streaming multiplexed by stream_id)
  - Raw calls (no protobuf user payload encoding/decoding, direct bytes transmission — use for custom serialization)
- Cross-language interoperability: Rust ↔ Go ↔ Node.js (Node.js uses napi-rs bindings to Rust implementation)

## Directory Structure

- `rust/`: Rust implementation (crate: `forpc`)
- `go/`: Go implementation (module: `github.com/bytemain/forpc/go`)
- `node/`: Node.js native extension (napi-rs, bindings to Rust `forpc`)
- `scripts/interop_matrix.sh`: Interoperability matrix test (client/server Cartesian product)
- `docs/TECHNICAL_SPECIFICATION_CN.md`: Protocol and implementation details

## Quick Verification (Recommended)

Run the interoperability matrix directly:

```bash
bash scripts/interop_matrix.sh
```

This script will build Rust/Go examples, build the Node addon, and run through:

- Rust server → Go client
- Go server → Rust client
- Go server → Go client
- Rust server → Rust client
- Rust raw server → Node raw client
- Node raw server → Rust raw client
- Go raw server → Node raw client
- Node raw server → Go raw client
- Node raw server → Node raw client
- Rust msgpack server → Go msgpack client (cross-language MessagePack)
- Go msgpack server → Rust msgpack client
- Rust msgpack server → Node msgpack client
- Node msgpack server → Rust msgpack client
- Go msgpack server → Node msgpack client
- Node msgpack server → Go msgpack client

## Running Examples Manually

### Rust server → Go client (Unary)

```bash
cd rust
cargo run --example interop_echo_server -- tcp://127.0.0.1:24000
```

In another terminal:

```bash
cd go
go run ./examples/interop_echo_client --connect tcp://127.0.0.1:24000 --msg Hello
```

### Node server (JS handler) → Rust client (Raw)

```bash
cd node
npm run build:debug
node scripts/interop_raw_echo_server.js tcp://127.0.0.1:24002 Raw/Echo
```

In another terminal:

```bash
cd rust
cargo run --example interop_raw_echo_client -- tcp://127.0.0.1:24002 Raw/Echo HelloNode
```

## Custom Serialization (MessagePack Example)

forpc uses Protocol Buffers by default for the typed `call()` API, but also provides `callRaw()` / `CallRaw()` / `call_raw()` which transmits raw bytes without any framework-level serialization. This makes it straightforward to use **any** serialization format — MessagePack, JSON, FlatBuffers, Cap'n Proto, or your own binary protocol — while still benefiting from forpc's transport, framing, multiplexing, and cross-language interop.

### How it works

```
┌──────────┐     callRaw(method, bytes)     ┌──────────┐
│  Client   │ ────────────────────────────►  │  Server   │
│           │                                │           │
│ encode()  │   forpc handles framing,       │ decode()  │
│ decode()  │   routing, multiplexing        │ encode()  │
│           │  ◄──────────────────────────── │           │
└──────────┘     raw bytes response          └──────────┘
```

Your application serializes/deserializes the payload; forpc handles everything else (transport, stream multiplexing, metadata, timeouts, error propagation).

### MessagePack libraries

| Language | Library | Install |
|----------|---------|---------|
| Rust | [`rmp-serde`](https://crates.io/crates/rmp-serde) | `cargo add rmp-serde` |
| Go | [`github.com/vmihailenco/msgpack/v5`](https://github.com/vmihailenco/msgpack) | `go get github.com/vmihailenco/msgpack/v5` |
| Node.js | [`@msgpack/msgpack`](https://www.npmjs.com/package/@msgpack/msgpack) | `npm install @msgpack/msgpack` |

### Rust example

```rust
use bytes::Bytes;
use forpc::RpcPeer;

#[derive(serde::Serialize, serde::Deserialize)]
struct EchoMsg { message: String }

// Client: encode with MessagePack, send via call_raw
let req = EchoMsg { message: "Hello".into() };
let payload = rmp_serde::to_vec_named(&req)?;  // use to_vec_named for cross-language compat
let resp = peer.call_raw("MsgPack/Echo", Bytes::from(payload)).await?;
let reply: EchoMsg = rmp_serde::from_slice(&resp)?;
println!("reply: {}", reply.message);
```

### Go example

```go
import "github.com/vmihailenco/msgpack/v5"

type EchoMsg struct {
    Message string `msgpack:"message"`
}

// Client: encode with MessagePack, send via CallRaw
payload, _ := msgpack.Marshal(&EchoMsg{Message: "Hello"})
resp, _ := peer.CallRaw("MsgPack/Echo", payload)
var reply EchoMsg
msgpack.Unmarshal(resp, &reply)
fmt.Println("reply:", reply.Message)
```

### Node.js example

```js
const { encode, decode } = require('@msgpack/msgpack')

// Client: encode with MessagePack, send via callRaw
const resp = await peer.callRaw('MsgPack/Echo', Buffer.from(encode({ message: 'Hello' })))
const reply = decode(resp)
console.log('reply:', reply.message)
```

### Running the MessagePack examples

Start a server in one language and connect a client in another — MessagePack is a cross-language binary format, so all combinations work:

```bash
# Go server
cd go && go run ./examples/msgpack_echo_server --listen tcp://127.0.0.1:24010

# Rust client (in another terminal)
cd rust && cargo run --example msgpack_echo_client -- tcp://127.0.0.1:24010 MsgPack/Echo "Hello from Rust"
```

```bash
# Rust server
cd rust && cargo run --example msgpack_echo_server -- tcp://127.0.0.1:24010

# Node.js client (in another terminal)
cd node && node scripts/msgpack_echo_client.js tcp://127.0.0.1:24010 MsgPack/Echo "Hello from Node"
```

Full working examples are available in each language directory:
- Rust: `rust/examples/msgpack_echo_server.rs`, `rust/examples/msgpack_echo_client.rs`
- Go: `go/examples/msgpack_echo_server/`, `go/examples/msgpack_echo_client/`
- Node.js: `node/scripts/msgpack_echo_server.js`, `node/scripts/msgpack_echo_client.js`

### Using other serialization formats

The same `callRaw` pattern works with any serialization format. Simply replace the encode/decode calls:

| Format | Rust | Go | Node.js |
|--------|------|-----|---------|
| **MessagePack** | `rmp-serde` | `vmihailenco/msgpack/v5` | `@msgpack/msgpack` |
| **JSON** | `serde_json` | `encoding/json` | `JSON.stringify/parse` |
| **FlatBuffers** | `flatbuffers` | `google/flatbuffers` | `flatbuffers` |
| **Cap'n Proto** | `capnp` | `capnproto/go-capnp` | `capnp-ts` |
| **CBOR** | `ciborium` | `fxamacker/cbor/v2` | `cbor-x` |
| **Custom binary** | Manual `bytes` | Manual `[]byte` | Manual `Buffer` |

## Regenerating Protobuf Code

The generated protobuf Rust code is pre-committed in `rust/src/gen/`. Normal builds do not require `protoc`. To regenerate after `.proto` file changes:

```bash
cd rust
FORPC_GENERATE_PROTO=1 cargo build
```

This requires `protoc` to be installed. See [protobuf install instructions](https://github.com/protocolbuffers/protobuf#protobuf-compiler-installation).

## Notes

- The current implementation prioritizes "interoperability/extensibility" and is still under rapid iteration; refer to `docs/TECHNICAL_SPECIFICATION_CN.md` for protocol details.

