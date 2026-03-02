# forpc
<p align="center">
  <a href="./README_zh.md">简体中文</a>
</p>

forpc is a lightweight RPC/Streaming framework based on NNG (nanomsg-next-gen) transport, using Protocol Buffers (protobuf) for cross-language serialization. Its goal is to enable seamless communication between Rust/Go/Node.js using a unified protocol.

## Features

- Transport: NNG
  - Rust: `anng` (nng-rs)
  - Go: `mangos` (nanomsg over Go)
- Serialization: Protocol Buffers (protobuf)
  - Rust: `prost` crate with derive macros
  - Go: `google.golang.org/protobuf`
- Call Models
  - Unary (single request/single response)
  - Bidi streaming (DATA/TRAILERS frame streaming multiplexed by stream_id)
  - Raw calls (no protobuf user payload encoding/decoding, direct bytes transmission)
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

## Notes

- The current implementation prioritizes "interoperability/extensibility" and is still under rapid iteration; refer to `docs/TECHNICAL_SPECIFICATION_CN.md` for protocol details.

