/**
 * Peer - RPC client for forpc
 *
 * Wraps the low-level AsyncDealer transport with the forpc
 * RPC protocol (Packet framing + protobuf Call/Status messages).
 *
 * Enables Node.js to invoke RPC methods on Rust/Go servers and vice versa.
 *
 * Provides both raw byte calls (callRaw) and typed protobuf calls (call),
 * following the same API patterns as Rust and Go implementations
 * (similar to gRPC/bRPC calling conventions).
 */

import { AsyncDealer } from '../transport/index.js'
import {
  type Call,
  type Status,
  StatusCode,
  FrameKind,
  encodePacket,
  decodePacket,
  headersPacket,
  dataPacket,
  trailersPacket,
  statusOk,
  decodeStatus,
} from './protocol.ts'

export class RpcError extends Error {
  code: number

  constructor(code: number, message: string) {
    super(message)
    this.code = code
    this.name = 'RpcError'
  }
}

/**
 * Protobuf message type interface.
 *
 * Compatible with protobufjs generated message classes
 * (e.g. `root.lookupType('package.MessageName')`).
 */
export interface MessageType<T = object> {
  encode(message: T): { finish(): Uint8Array }
  decode(buf: Uint8Array): T
  create(properties?: Partial<T>): T
}

/**
 * Service definition for gRPC-style typed calls.
 *
 * Maps method names to their request/response protobuf message types,
 * following the same pattern as gRPC service definitions.
 *
 * @example
 * ```ts
 * const echoService: ServiceDefinition = {
 *   Echo: {
 *     requestType: EchoRequest,
 *     responseType: EchoResponse,
 *   },
 * }
 * ```
 */
export interface ServiceDefinition {
  [methodName: string]: {
    requestType: MessageType
    responseType: MessageType
  }
}

/**
 * Peer - Client-side RPC peer
 *
 * Connects to a remote RPC server and can invoke unary calls.
 * Uses AsyncDealer for the underlying transport.
 *
 * Supports two calling modes:
 * - **callRaw**: Low-level raw byte calls (compatible with any payload format)
 * - **call**: Typed protobuf calls with automatic serialization (like gRPC/bRPC)
 *
 * @example
 * ```ts
 * const peer = await Peer.connect('tcp://127.0.0.1:24000')
 *
 * // Raw call
 * const resp = await peer.callRaw('Echo/Ping', Buffer.from('hello'))
 *
 * // Typed protobuf call
 * const reply = await peer.call('Echo/Ping', EchoRequest, EchoResponse, { data: 'hello' })
 * ```
 */
export class Peer {
  private dealer: AsyncDealer
  private nextStreamId: number

  private constructor(dealer: AsyncDealer) {
    this.dealer = dealer
    this.nextStreamId = 1
  }

  /**
   * Connect to a remote RPC server
   */
  static async connect(url: string): Promise<Peer> {
    const dealer = await AsyncDealer.dial(url)
    return new Peer(dealer)
  }

  /**
   * Close the peer and cancel pending operations
   */
  close(): void {
    this.dealer.close()
  }

  private allocStreamId(): number {
    const id = this.nextStreamId
    this.nextStreamId += 2
    return id
  }

  /**
   * Make a raw unary RPC call (no protobuf serialization of request/response payload).
   *
   * Sends HEADERS, DATA, and TRAILERS packets, then receives
   * the response DATA and TRAILERS packets sequentially.
   */
  async callRaw(method: string, payload: Buffer, metadata?: Record<string, string>): Promise<Buffer> {
    const streamId = this.allocStreamId()
    const call: Call = { method, metadata: metadata || {} }

    // Send HEADERS
    await this.dealer.send(encodePacket(headersPacket(streamId, call)))

    // Send DATA
    await this.dealer.send(encodePacket(dataPacket(streamId, payload)))

    // Send TRAILERS (OK - indicating end of client stream)
    await this.dealer.send(encodePacket(trailersPacket(streamId, statusOk())))

    // Receive response packets until we get TRAILERS
    let resultData: Buffer | undefined
    while (true) {
      const msg = await this.dealer.recv()
      const body = msg.body()
      if (body.length < 5) continue

      const packet = decodePacket(body)

      if (packet.kind === FrameKind.DATA) {
        resultData = packet.payload
      } else if (packet.kind === FrameKind.TRAILERS) {
        const status: Status = decodeStatus(packet.payload)
        if (status.code === StatusCode.OK) {
          return resultData || Buffer.alloc(0)
        } else {
          throw new RpcError(status.code, status.message)
        }
      }
    }
  }

  /**
   * Make a raw unary RPC call with explicit metadata.
   *
   * Convenience alias matching the Rust/Go `call_raw_with_metadata` / `CallRawWithMetadata` API.
   */
  async callRawWithMetadata(
    method: string,
    payload: Buffer,
    metadata: Record<string, string>,
  ): Promise<Buffer> {
    return this.callRaw(method, payload, metadata)
  }

  /**
   * Make a typed unary RPC call with automatic protobuf serialization.
   *
   * Encodes the request using `requestType`, sends it, and decodes
   * the response using `responseType` — matching the Rust `call<Req, Resp>()`
   * and Go `Call(method, req, resp)` patterns.
   *
   * @param method - RPC method name (e.g. "Service/Method")
   * @param requestType - Protobuf message type for the request
   * @param responseType - Protobuf message type for the response
   * @param request - Request payload object (will be encoded via requestType)
   * @param metadata - Optional per-call metadata key-value pairs
   * @returns Decoded response object
   *
   * @example
   * ```ts
   * const resp = await peer.call(
   *   'Test/Echo',
   *   EchoRequest,
   *   EchoResponse,
   *   { data: 'hello' },
   * )
   * console.log(resp.result) // 'hello'
   * ```
   */
  async call<Req extends object, Resp extends object>(
    method: string,
    requestType: MessageType<Req>,
    responseType: MessageType<Resp>,
    request: Partial<Req>,
    metadata?: Record<string, string>,
  ): Promise<Resp> {
    const reqMsg = requestType.create(request)
    const payload = Buffer.from(requestType.encode(reqMsg).finish())
    const respBuf = await this.callRaw(method, payload, metadata)
    return responseType.decode(respBuf)
  }
}
