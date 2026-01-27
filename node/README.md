# forpc
<p align="center">
  <a href="./README_zh.md">简体中文</a>
</p>

`forpc` is a high-performance Node.js native module built with [napi-rs](https://napi.rs/), providing asynchronous bindings for [nng](https://nng.nanomsg.org/) (nanomsg next generation). It focuses on providing high-performance, low-latency asynchronous network communication.

## Features

- **High Performance**: Written in Rust, called with zero overhead via Node-API.
- **Fully Asynchronous**: Deeply integrated with the Node.js event loop; all network IO operations are non-blocking and asynchronous.
- **REQ/REP Pattern**: Provides implementation for `AsyncDealer` (client) and `AsyncRouter` (server).
- **Multi-platform Support**: Pre-compiled support for Windows, macOS, Linux (x86_64 and arm64).

## Installation

```bash
npm install forpc
```

## Quick Start

### Client (AsyncDealer)

```javascript
import { AsyncDealer } from 'forpc'

async function client() {
  const dealer = await AsyncDealer.dial('tcp://127.0.0.1:5555')
  
  // Send request
  const requestId = await dealer.send(Buffer.from('Hello from client'))
  console.log(`Request sent, ID: ${requestId}`)
  
  // Receive response
  const msg = await dealer.recv()
  console.log('Received:', msg.body().toString())
}

client()
```

### Server (AsyncRouter)

```javascript
import { AsyncRouter } from 'forpc'

async function server() {
  const router = await AsyncRouter.listen('tcp://127.0.0.1:5555')
  console.log('Server listening on tcp://127.0.0.1:5555')

  while (true) {
    // Receive request
    const req = await router.recv()
    console.log('Received:', req.body().toString())

    // Create response
    const res = req.createResponse(Buffer.from('Hello from server'))
    
    // Send response
    await router.send(res)
  }
}

server()
```

## API Documentation

### `AsyncDealer`
- `static dial(url: string): Promise<AsyncDealer>`: Connects to the specified URL.
- `send(body: Buffer): Promise<number>`: Sends a message and returns the request ID.
- `recv(): Promise<DealerMessage>`: Asynchronously receives a message.

### `AsyncRouter`
- `static listen(url: string): Promise<AsyncRouter>`: Listens on the specified URL.
- `recv(): Promise<RouterMessage>`: Asynchronously receives a message from a client.
- `send(msg: RouterMessage): Promise<void>`: Sends a response message back to the client.

### `RouterMessage`
- `identity(): Buffer | null`: Gets the client identity.
- `body(): Buffer`: Gets the message body.
- `createResponse(body: Buffer): RouterMessage`: Creates a response message with the same header (containing routing information) but a new body.

## Local Development

### Requirements
- [Rust](https://www.rust-lang.org/) (latest stable version)
- [Node.js](https://nodejs.org/) (v12.22.0+)

### Build and Test
```bash
# Install dependencies
npm install

# Build local binary (Debug)
npm run build:debug

# Build release version
npm run build

# Run tests
npm test
```

## License

[MIT](./LICENSE)
