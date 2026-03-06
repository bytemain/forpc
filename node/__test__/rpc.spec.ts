import test from 'ava'
import path from 'path'
import { fileURLToPath } from 'url'
import protobuf from 'protobufjs'
import { encode, decode } from '@msgpack/msgpack'

import { Peer, RpcError, RawServer } from '../index'
import type { MessageType, ServiceDefinition } from '../index'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

test('Peer and RawServer raw echo communication', async (t) => {
  const url = `inproc://rpc_test_${process.pid}_${Date.now()}`

  // Start the server
  const server = await RawServer.bind(url)
  server.register('Raw/Echo', (payload) => {
    return Buffer.from(payload)
  })
  server.serve()

  // Connect the client
  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  // Make an RPC call
  const response = await peer.callRaw('Raw/Echo', Buffer.from('Hello RPC'))
  t.deepEqual(response, Buffer.from('Hello RPC'))
})

test('Peer and RawServer with metadata', async (t) => {
  const url = `inproc://rpc_meta_test_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Test/WithMeta', (payload, metadata) => {
    const msg = payload.toString()
    const prefix = metadata['prefix'] || ''
    return Buffer.from(prefix + msg)
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const response = await peer.callRaw('Test/WithMeta', Buffer.from('World'), { prefix: 'Hello ' })
  t.is(response.toString(), 'Hello World')
})

test('RpcError has correct properties', (t) => {
  const err = new RpcError(13, 'internal error')
  t.is(err.code, 13)
  t.is(err.message, 'internal error')
  t.is(err.name, 'RpcError')
  t.true(err instanceof Error)
})

test('Peer gets error for unregistered method', async (t) => {
  const url = `inproc://rpc_unregistered_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Exists/Method', () => Buffer.from('ok'))
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const err = await t.throwsAsync(() => peer.callRaw('NotExists/Method', Buffer.from('test')))
  t.truthy(err)
  t.true(err instanceof RpcError)
  if (err instanceof RpcError) {
    t.is(err.code, 12) // UNIMPLEMENTED
  }
})

// --- Tests for typed protobuf API (gRPC-style) ---

let EchoRequest: MessageType<{ data: string }>
let EchoResponse: MessageType<{ result: string }>

test.before(async () => {
  const root = await protobuf.load(path.join(__dirname, '../../proto/forpc_test.proto'))
  EchoRequest = root.lookupType('forpc.test.EchoRequest') as unknown as MessageType<{ data: string }>
  EchoResponse = root.lookupType('forpc.test.EchoResponse') as unknown as MessageType<{ result: string }>
})

test('Peer.call with typed protobuf echo', async (t) => {
  const url = `inproc://rpc_typed_echo_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.registerUnary('Test/Echo', EchoRequest, EchoResponse, (req) => {
    return { result: req.data }
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.call('Test/Echo', EchoRequest, EchoResponse, { data: 'hello typed' })
  t.is(resp.result, 'hello typed')
})

test('Peer.call with typed protobuf and metadata', async (t) => {
  const url = `inproc://rpc_typed_meta_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.registerUnary('Test/EchoMeta', EchoRequest, EchoResponse, (req, metadata) => {
    const prefix = metadata['prefix'] || ''
    return { result: prefix + req.data }
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.call('Test/EchoMeta', EchoRequest, EchoResponse, { data: 'World' }, { prefix: 'Hello ' })
  t.is(resp.result, 'Hello World')
})

test('Peer.callRawWithMetadata works like callRaw with metadata', async (t) => {
  const url = `inproc://rpc_rawmeta_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Test/RawMeta', (payload, metadata) => {
    const prefix = metadata['tag'] || ''
    return Buffer.from(prefix + payload.toString())
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.callRawWithMetadata('Test/RawMeta', Buffer.from('data'), { tag: '[INFO] ' })
  t.is(resp.toString(), '[INFO] data')
})

test('RawServer.addService registers all methods of a service', async (t) => {
  const url = `inproc://rpc_service_${process.pid}_${Date.now()}`

  const echoServiceDef: ServiceDefinition = {
    Echo: { requestType: EchoRequest, responseType: EchoResponse },
  }

  const server = await RawServer.bind(url)
  server.addService('Test', echoServiceDef, {
    Echo: (req: { data: string }) => ({ result: req.data }),
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.call('Test/Echo', EchoRequest, EchoResponse, { data: 'service hello' })
  t.is(resp.result, 'service hello')
})

test('RawServer.addService skips methods without handlers', async (t) => {
  const url = `inproc://rpc_svc_skip_${process.pid}_${Date.now()}`

  const serviceDef: ServiceDefinition = {
    Echo: { requestType: EchoRequest, responseType: EchoResponse },
    Missing: { requestType: EchoRequest, responseType: EchoResponse },
  }

  const server = await RawServer.bind(url)
  server.addService('Svc', serviceDef, {
    Echo: (req: { data: string }) => ({ result: req.data }),
    // Missing handler intentionally omitted
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  // Registered method works
  const resp = await peer.call('Svc/Echo', EchoRequest, EchoResponse, { data: 'ok' })
  t.is(resp.result, 'ok')

  // Missing method returns UNIMPLEMENTED
  const err = await t.throwsAsync(() => peer.call('Svc/Missing', EchoRequest, EchoResponse, { data: 'fail' }))
  t.truthy(err)
  t.true(err instanceof RpcError)
  if (err instanceof RpcError) {
    t.is(err.code, 12) // UNIMPLEMENTED
  }
})

test('registerUnary with async handler', async (t) => {
  const url = `inproc://rpc_async_unary_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.registerUnary('Test/AsyncEcho', EchoRequest, EchoResponse, async (req) => {
    // Simulate async work
    await new Promise((resolve) => setTimeout(resolve, 10))
    return { result: `async: ${req.data}` }
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.call('Test/AsyncEcho', EchoRequest, EchoResponse, { data: 'test' })
  t.is(resp.result, 'async: test')
})

// --- Tests for concurrent calls (matching Rust/Go unlimited concurrent call/recv) ---

test('concurrent raw calls with Promise.all', async (t) => {
  const url = `inproc://rpc_concurrent_raw_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Raw/Echo', (payload) => {
    return Buffer.from(payload)
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  // Fire multiple calls concurrently
  const results = await Promise.all([
    peer.callRaw('Raw/Echo', Buffer.from('msg-1')),
    peer.callRaw('Raw/Echo', Buffer.from('msg-2')),
    peer.callRaw('Raw/Echo', Buffer.from('msg-3')),
    peer.callRaw('Raw/Echo', Buffer.from('msg-4')),
    peer.callRaw('Raw/Echo', Buffer.from('msg-5')),
  ])

  t.is(results.length, 5)
  t.deepEqual(results[0], Buffer.from('msg-1'))
  t.deepEqual(results[1], Buffer.from('msg-2'))
  t.deepEqual(results[2], Buffer.from('msg-3'))
  t.deepEqual(results[3], Buffer.from('msg-4'))
  t.deepEqual(results[4], Buffer.from('msg-5'))
})

test('concurrent typed calls with Promise.all', async (t) => {
  const url = `inproc://rpc_concurrent_typed_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.registerUnary('Test/Echo', EchoRequest, EchoResponse, (req) => {
    return { result: req.data }
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const results = await Promise.all([
    peer.call('Test/Echo', EchoRequest, EchoResponse, { data: 'a' }),
    peer.call('Test/Echo', EchoRequest, EchoResponse, { data: 'b' }),
    peer.call('Test/Echo', EchoRequest, EchoResponse, { data: 'c' }),
  ])

  t.is(results.length, 3)
  t.is(results[0].result, 'a')
  t.is(results[1].result, 'b')
  t.is(results[2].result, 'c')
})

test('sequential calls after concurrent batch', async (t) => {
  const url = `inproc://rpc_seq_after_conc_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Raw/Echo', (payload) => Buffer.from(payload))
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  // First batch: concurrent
  const batch1 = await Promise.all([
    peer.callRaw('Raw/Echo', Buffer.from('batch1-a')),
    peer.callRaw('Raw/Echo', Buffer.from('batch1-b')),
  ])
  t.deepEqual(batch1[0], Buffer.from('batch1-a'))
  t.deepEqual(batch1[1], Buffer.from('batch1-b'))

  // Then sequential
  const r1 = await peer.callRaw('Raw/Echo', Buffer.from('seq-1'))
  t.deepEqual(r1, Buffer.from('seq-1'))

  // Another concurrent batch
  const batch2 = await Promise.all([
    peer.callRaw('Raw/Echo', Buffer.from('batch2-a')),
    peer.callRaw('Raw/Echo', Buffer.from('batch2-b')),
    peer.callRaw('Raw/Echo', Buffer.from('batch2-c')),
  ])
  t.deepEqual(batch2[0], Buffer.from('batch2-a'))
  t.deepEqual(batch2[1], Buffer.from('batch2-b'))
  t.deepEqual(batch2[2], Buffer.from('batch2-c'))
})

// --- Tests for timeout/deadline enforcement ---

test('callRaw with :timeout metadata triggers DEADLINE_EXCEEDED on slow handler', async (t) => {
  const url = `inproc://rpc_timeout_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Test/Slow', async () => {
    // Simulate slow handler
    await new Promise((resolve) => setTimeout(resolve, 5000))
    return Buffer.from('late')
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const err = await t.throwsAsync(() =>
    peer.callRaw('Test/Slow', Buffer.from('hello'), { ':timeout': '100' }),
  )
  t.truthy(err)
  t.true(err instanceof RpcError)
  if (err instanceof RpcError) {
    t.is(err.code, 4) // DEADLINE_EXCEEDED
  }
})

test('callRaw completes before timeout', async (t) => {
  const url = `inproc://rpc_timeout_ok_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('Raw/Echo', (payload) => Buffer.from(payload))
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const resp = await peer.callRaw('Raw/Echo', Buffer.from('fast'), { ':timeout': '5000' })
  t.deepEqual(resp, Buffer.from('fast'))
})

test('Peer and RawServer with MessagePack encoding', async (t) => {
  const url = `inproc://rpc_msgpack_${process.pid}_${Date.now()}`

  const server = await RawServer.bind(url)
  server.register('MsgPack/Echo', (payload) => {
    const decoded = decode(payload) as { message: string }
    return Buffer.from(encode({ message: decoded.message }))
  })
  server.serve()

  const peer = await Peer.connect(url)

  t.teardown(() => {
    peer.close()
    server.close()
  })

  const request = Buffer.from(encode({ message: 'Hello MsgPack' }))
  const response = await peer.callRaw('MsgPack/Echo', request)
  const result = decode(response) as { message: string }
  t.is(result.message, 'Hello MsgPack')
})
