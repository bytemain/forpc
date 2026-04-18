/**
 * Protocol layer for forpc
 *
 * The wire format is a single protobuf-encoded `Packet` message. There is
 * no hand-written framing on top: encode/decode round-trip exclusively
 * through `Packet.encode` / `Packet.decode` from the generated protobuf
 * code, matching the Rust and Go implementations.
 *
 * Re-exports protobuf-generated `Call`, `Status`, `Packet` and `FrameKind`
 * symbols and provides small ergonomic helpers used by `peer.ts` /
 * `server.ts`.
 */

import proto from './generated/forpc.js'

const CallMessage = proto.forpc.Call
const StatusMessage = proto.forpc.Status
const PacketMessage = proto.forpc.Packet

// Frame kind constants, sourced from the generated protobuf enum.
export const FrameKind = proto.forpc.FrameKind

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
  errorCode: number
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
 * Encode a Packet to its protobuf wire bytes.
 */
export function encodePacket(packet: Packet): Buffer {
  const msg = PacketMessage.create({
    streamId: packet.streamId,
    kind: packet.kind,
    payload: packet.payload,
    errorCode: packet.errorCode,
  })
  return Buffer.from(PacketMessage.encode(msg).finish())
}

/**
 * Decode protobuf wire bytes into a Packet.
 */
export function decodePacket(data: Buffer): Packet {
  const msg = PacketMessage.decode(data) as unknown as {
    streamId: number
    kind: number
    payload: Uint8Array
    errorCode: number
  }
  return {
    streamId: msg.streamId || 0,
    kind: msg.kind || 0,
    payload: Buffer.from(msg.payload || []),
    errorCode: msg.errorCode || 0,
  }
}

/**
 * Create a HEADERS packet with an encoded Call
 */
export function headersPacket(streamId: number, call: Call): Packet {
  return {
    streamId,
    kind: FrameKind.HEADERS,
    payload: encodeCall(call),
    errorCode: 0,
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
    errorCode: 0,
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
    errorCode: 0,
  }
}

/**
 * Create a RST_STREAM packet with an error code carried in the dedicated
 * `errorCode` field of the protobuf message.
 */
export function rstStreamPacket(streamId: number, errorCode: number): Packet {
  return {
    streamId,
    kind: FrameKind.RST_STREAM,
    payload: Buffer.alloc(0),
    errorCode,
  }
}

/**
 * Create an OK status
 */
export function statusOk(): Status {
  return { code: StatusCode.OK, message: 'OK' }
}
