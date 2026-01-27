# forpc

`forpc` 是一个基于 [napi-rs](https://napi.rs/) 构建的高性能 Node.js 原生模块，提供了对 [nng](https://nng.nanomsg.org/) (nanomsg next generation) 的异步绑定。它专注于提供高性能、低延迟的异步网络通信能力。

## 特性

- **高性能**: 使用 Rust 编写，通过 Node-API 零开销调用。
- **全异步**: 深度集成 Node.js 事件循环，所有的网络 IO 操作均为非阻塞异步。
- **REQ/REP 模式**: 提供了 `AsyncDealer` (客户端) 和 `AsyncRouter` (服务端) 实现。
- **多平台支持**: 预编译支持 Windows, macOS, Linux (x86_64 和 arm64)。

## 安装

```bash
npm install forpc
```

## 快速开始

### 客户端 (AsyncDealer)

```javascript
import { AsyncDealer } from 'forpc'

async function client() {
  const dealer = await AsyncDealer.dial('tcp://127.0.0.1:5555')
  
  // 发送请求
  const requestId = await dealer.send(Buffer.from('Hello from client'))
  console.log(`Request sent, ID: ${requestId}`)
  
  // 接收响应
  const msg = await dealer.recv()
  console.log('Received:', msg.body().toString())
}

client()
```

### 服务端 (AsyncRouter)

```javascript
import { AsyncRouter } from 'forpc'

async function server() {
  const router = await AsyncRouter.listen('tcp://127.0.0.1:5555')
  console.log('Server listening on tcp://127.0.0.1:5555')

  while (true) {
    // 接收请求
    const req = await router.recv()
    console.log('Received:', req.body().toString())

    // 创建响应
    const res = req.createResponse(Buffer.from('Hello from server'))
    
    // 发送响应
    await router.send(res)
  }
}

server()
```

## API 说明

### `AsyncDealer`
- `static dial(url: string): Promise<AsyncDealer>`: 连接到指定地址。
- `send(body: Buffer): Promise<number>`: 发送消息，返回请求 ID。
- `recv(): Promise<DealerMessage>`: 异步接收消息。

### `AsyncRouter`
- `static listen(url: string): Promise<AsyncRouter>`: 监听指定地址。
- `recv(): Promise<RouterMessage>`: 异步接收来自客户端的消息。
- `send(msg: RouterMessage): Promise<void>`: 发送响应消息。

### `RouterMessage`
- `identity(): Buffer | null`: 获取客户端标识。
- `body(): Buffer`: 获取消息体。
- `createResponse(body: Buffer): RouterMessage`: 根据当前消息的 Header（包含路由信息）创建一个响应消息。

## 本地开发

### 要求
- [Rust](https://www.rust-lang.org/) (最新稳定版)
- [Node.js](https://nodejs.org/) (v12.22.0+)

### 构建与测试
```bash
# 安装依赖
npm install

# 构建本地二进制文件 (Debug)
npm run build:debug

# 构建发布版本
npm run build

# 运行测试
npm test
```

## 许可证

[MIT](./LICENSE)
