import test from 'ava'

import { Peer, RpcError, RawServer } from '../index'

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

  // Make an RPC call
  const response = await peer.callRaw('Raw/Echo', Buffer.from('Hello RPC'))
  t.deepEqual(response, Buffer.from('Hello RPC'))

  t.teardown(() => {
    peer.close()
    server.close()
  })
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
  const response = await peer.callRaw('Test/WithMeta', Buffer.from('World'), { prefix: 'Hello ' })
  t.is(response.toString(), 'Hello World')

  t.teardown(() => {
    peer.close()
    server.close()
  })
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

  const err = await t.throwsAsync(() => peer.callRaw('NotExists/Method', Buffer.from('test')))
  t.truthy(err)
  t.true(err instanceof RpcError)
  if (err instanceof RpcError) {
    t.is(err.code, 12) // UNIMPLEMENTED
  }

  t.teardown(() => {
    peer.close()
    server.close()
  })
})
