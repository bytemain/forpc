import test from 'ava'

import {
  FrameKind,
  StatusCode,
  encodePacket,
  decodePacket,
  encodeCall,
  decodeCall,
  encodeStatus,
  decodeStatus,
  headersPacket,
  dataPacket,
  trailersPacket,
  statusOk,
} from '../src/protocol'

test('FrameKind constants match Rust/Go', (t) => {
  t.is(FrameKind.HEADERS, 0)
  t.is(FrameKind.DATA, 1)
  t.is(FrameKind.TRAILERS, 2)
})

test('StatusCode constants match Rust/Go', (t) => {
  t.is(StatusCode.OK, 0)
  t.is(StatusCode.CANCELLED, 1)
  t.is(StatusCode.UNKNOWN, 2)
  t.is(StatusCode.INVALID_ARGUMENT, 3)
  t.is(StatusCode.UNIMPLEMENTED, 12)
  t.is(StatusCode.INTERNAL, 13)
  t.is(StatusCode.UNAVAILABLE, 14)
})

test('Packet encode/decode roundtrip', (t) => {
  const packet = { streamId: 123, kind: FrameKind.DATA, payload: Buffer.from('hello') }
  const encoded = encodePacket(packet)
  t.is(encoded.length, 5 + 5) // 4 bytes streamId + 1 byte kind + 5 bytes payload
  const decoded = decodePacket(encoded)
  t.is(decoded.streamId, 123)
  t.is(decoded.kind, FrameKind.DATA)
  t.deepEqual(decoded.payload, Buffer.from('hello'))
})

test('Packet decode rejects short data', (t) => {
  t.throws(() => decodePacket(Buffer.from([1, 2, 3, 4])), {
    message: /packet too short/,
  })
})

test('Packet with empty payload', (t) => {
  const packet = { streamId: 1, kind: FrameKind.HEADERS, payload: Buffer.alloc(0) }
  const encoded = encodePacket(packet)
  t.is(encoded.length, 5)
  const decoded = decodePacket(encoded)
  t.is(decoded.streamId, 1)
  t.is(decoded.kind, FrameKind.HEADERS)
  t.is(decoded.payload.length, 0)
})

test('Call encode/decode roundtrip', (t) => {
  const call = { method: 'Test/Echo', metadata: { key: 'value', foo: 'bar' } }
  const encoded = encodeCall(call)
  const decoded = decodeCall(encoded)
  t.is(decoded.method, 'Test/Echo')
  t.is(decoded.metadata['key'], 'value')
  t.is(decoded.metadata['foo'], 'bar')
})

test('Call with empty metadata', (t) => {
  const call = { method: 'Test/Method', metadata: {} }
  const encoded = encodeCall(call)
  const decoded = decodeCall(encoded)
  t.is(decoded.method, 'Test/Method')
  t.deepEqual(decoded.metadata, {})
})

test('Status encode/decode roundtrip', (t) => {
  const status = { code: StatusCode.OK, message: 'OK' }
  const encoded = encodeStatus(status)
  const decoded = decodeStatus(encoded)
  t.is(decoded.code, 0)
  t.is(decoded.message, 'OK')
})

test('Status with error code', (t) => {
  const status = { code: StatusCode.INTERNAL, message: 'Something went wrong' }
  const encoded = encodeStatus(status)
  const decoded = decodeStatus(encoded)
  t.is(decoded.code, 13)
  t.is(decoded.message, 'Something went wrong')
})

test('statusOk helper', (t) => {
  const status = statusOk()
  t.is(status.code, StatusCode.OK)
  t.is(status.message, 'OK')
})

test('headersPacket helper', (t) => {
  const call = { method: 'Test/Echo', metadata: {} }
  const packet = headersPacket(1, call)
  t.is(packet.streamId, 1)
  t.is(packet.kind, FrameKind.HEADERS)
  const decoded = decodeCall(packet.payload)
  t.is(decoded.method, 'Test/Echo')
})

test('dataPacket helper', (t) => {
  const payload = Buffer.from('test data')
  const packet = dataPacket(2, payload)
  t.is(packet.streamId, 2)
  t.is(packet.kind, FrameKind.DATA)
  t.deepEqual(packet.payload, payload)
})

test('trailersPacket helper', (t) => {
  const status = { code: StatusCode.OK, message: 'OK' }
  const packet = trailersPacket(3, status)
  t.is(packet.streamId, 3)
  t.is(packet.kind, FrameKind.TRAILERS)
  const decoded = decodeStatus(packet.payload)
  t.is(decoded.code, 0)
  t.is(decoded.message, 'OK')
})

test('Call protobuf is compatible with Rust encoding', (t) => {
  // Verify the wire format is correct by checking that field tags match
  // In protobuf3: method=tag1, metadata=tag2
  const call = { method: 'A', metadata: {} }
  const encoded = encodeCall(call)
  // First byte should be 0x0a (field 1, wire type 2 = length-delimited)
  t.is(encoded[0], 0x0a)
})

test('Status protobuf is compatible with Rust encoding', (t) => {
  // code=tag1 (uint32), message=tag2 (string)
  const status = { code: 1, message: 'err' }
  const encoded = encodeStatus(status)
  // First byte should be 0x08 (field 1, wire type 0 = varint)
  t.is(encoded[0], 0x08)
})
