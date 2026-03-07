/**
 * Protocol layer for forpc
 *
 * Implements Packet framing and protobuf message encoding/decoding
 * that is compatible with the Rust and Go implementations.
 *
 * Packet wire format: [stream_id: u32 BE][kind: u8][payload...]
 * Protobuf messages: Call (method + metadata), Status (code + message)
 */

import proto from './generated/forpc.js'

const CallMessage = proto.forpc.Call
const StatusMessage = proto.forpc.Status

// Frame kind constants matching Rust/Go
export const FrameKind = {
  HEADERS: 0,
  DATA: 1,
  TRAILERS: 2,
  RST_STREAM: 3,
} as const

// gRPC-compatible status codes from protobuf definition
export const StatusCode = proto.forpc.StatusCode

export interface Call {
  method: string
  metadata: Record<string, string>
}

export interface Status {
  code: number
  message: string
}

export interface Packet {
  streamId: number
  kind: number
  payload: Buffer
}

/**
 * Encode a Call message to protobuf bytes
 */
export function encodeCall(call: Call): Buffer {
  const msg = CallMessage.create({
    method: call.method,
    metadata: call.metadata || {},
  })
  return Buffer.from(CallMessage.encode(msg).finish())
}

/**
 * Decode protobuf bytes to a Call message
 */
export function decodeCall(buf: Buffer): Call {
  const msg = CallMessage.decode(buf) as unknown as { method: string; metadata: Record<string, string> }
  return {
    method: msg.method || '',
    metadata: msg.metadata || {},
  }
}

/**
 * Encode a Status message to protobuf bytes
 */
export function encodeStatus(status: Status): Buffer {
  const msg = StatusMessage.create({
    code: status.code,
    message: status.message,
  })
  return Buffer.from(StatusMessage.encode(msg).finish())
}

/**
 * Decode protobuf bytes to a Status message
 */
export function decodeStatus(buf: Buffer): Status {
  const msg = StatusMessage.decode(buf) as unknown as { code: number; message: string }
  return {
    code: msg.code || 0,
    message: msg.message || '',
  }
}

/**
 * Encode a Packet to wire format: [stream_id: u32 BE][kind: u8][payload...]
 */
export function encodePacket(packet: Packet): Buffer {
  const buf = Buffer.alloc(5 + packet.payload.length)
  buf.writeUInt32BE(packet.streamId, 0)
  buf.writeUInt8(packet.kind, 4)
  packet.payload.copy(buf, 5)
  return buf
}

/**
 * Decode wire format bytes to a Packet
 */
export function decodePacket(data: Buffer): Packet {
  if (data.length < 5) {
    throw new Error(`packet too short: len=${data.length}`)
  }
  const streamId = data.readUInt32BE(0)
  const kind = data.readUInt8(4)
  const payload = data.subarray(5)
  return { streamId, kind, payload: Buffer.from(payload) }
}

/**
 * Create a HEADERS packet with an encoded Call
 */
export function headersPacket(streamId: number, call: Call): Packet {
  return {
    streamId,
    kind: FrameKind.HEADERS,
    payload: encodeCall(call),
  }
}

/**
 * Create a DATA packet
 */
export function dataPacket(streamId: number, payload: Buffer): Packet {
  return {
    streamId,
    kind: FrameKind.DATA,
    payload,
  }
}

/**
 * Create a TRAILERS packet with an encoded Status
 */
export function trailersPacket(streamId: number, status: Status): Packet {
  return {
    streamId,
    kind: FrameKind.TRAILERS,
    payload: encodeStatus(status),
  }
}

/**
 * Create a RST_STREAM packet with an error code (u32 BE)
 */
export function rstStreamPacket(streamId: number, errorCode: number): Packet {
  const payload = Buffer.alloc(4)
  payload.writeUInt32BE(errorCode, 0)
  return {
    streamId,
    kind: FrameKind.RST_STREAM,
    payload,
  }
}

/**
 * Create an OK status
 */
export function statusOk(): Status {
  return { code: StatusCode.OK, message: 'OK' }
}
