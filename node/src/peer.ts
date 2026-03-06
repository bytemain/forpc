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
 *
 * Uses a processing loop with stream-ID-based demultiplexing
 * to support unlimited concurrent calls, matching Rust/Go behavior.
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
 * Pending unary call state, matching Rust's PendingCall struct.
 *
 * Holds the Promise resolve/reject callbacks and buffered DATA payload
 * so the processing loop can deliver the result by stream ID.
 */
interface PendingCall {
  resolve: (data: Buffer) => void
  reject: (err: Error) => void
  dataBuffer?: Buffer
}

/**
 * Peer - Client-side RPC peer
 *
 * Connects to a remote RPC server and can invoke unary calls.
 * Uses AsyncDealer for the underlying transport.
 *
 * Supports unlimited concurrent calls via a processing loop
 * that demultiplexes responses by stream ID — matching the Rust/Go
 * architecture where a single `serve()` loop routes packets to
 * individual pending calls.
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
 * // Concurrent calls
 * const [r1, r2, r3] = await Promise.all([
 *   peer.callRaw('Echo/Ping', Buffer.from('a')),
 *   peer.callRaw('Echo/Ping', Buffer.from('b')),
 *   peer.callRaw('Echo/Ping', Buffer.from('c')),
 * ])
 *
 * // Typed protobuf call
 * const reply = await peer.call('Echo/Ping', EchoRequest, EchoResponse, { data: 'hello' })
 * ```
 */
export class Peer {
  private dealer: AsyncDealer
  private nextStreamId: number
  private pendingCalls: Map<number, PendingCall>
  private sendQueue: Buffer[]
  private loopRunning: boolean
  private closed: boolean

  private constructor(dealer: AsyncDealer) {
    this.dealer = dealer
    this.nextStreamId = 1
    this.pendingCalls = new Map()
    this.sendQueue = []
    this.loopRunning = false
    this.closed = false
  }

  /**
   * Connect to a remote RPC server
   */
  static async connect(url: string): Promise<Peer> {
    const dealer = await AsyncDealer.dial(url)
    return new Peer(dealer)
  }

  /**
   * Close the peer and cancel all pending operations.
   *
   * Any in-flight calls will be rejected with a CANCELLED error.
   */
  close(): void {
    this.closed = true
    this.dealer.close()
    // Reject all pending calls (matching Rust's behavior on shutdown)
    for (const [streamId, pending] of this.pendingCalls) {
      pending.reject(new RpcError(StatusCode.CANCELLED, 'Peer closed'))
      this.pendingCalls.delete(streamId)
    }
  }

  private allocStreamId(): number {
    const id = this.nextStreamId
    this.nextStreamId += 2
    return id
  }

  /**
   * Processing loop — matches Rust's `serve()` / Go's `Serve()`.
   *
   * Alternates between draining the send queue and receiving responses.
   * Routes received packets to the correct pending call by stream ID,
   * enabling unlimited concurrent calls.
   *
   * The loop runs only while there are pending calls or queued sends,
   * and restarts automatically when new calls are made.
   */
  private ensureLoop(): void {
    if (this.loopRunning) return
    this.loopRunning = true

    const loop = async () => {
      try {
        while (!this.closed) {
          // Phase 1: drain all queued sends
          while (this.sendQueue.length > 0) {
            const msg = this.sendQueue.shift()!
            await this.dealer.send(msg)
          }

          // Phase 2: if no pending calls, exit loop
          if (this.pendingCalls.size === 0) break

          // Phase 3: receive and route one response packet
          const msg = await this.dealer.recv()
          const body = msg.body()
          if (body.length < 5) continue

          const packet = decodePacket(body)
          const pending = this.pendingCalls.get(packet.streamId)
          if (!pending) continue

          if (packet.kind === FrameKind.DATA) {
            pending.dataBuffer = packet.payload
          } else if (packet.kind === FrameKind.TRAILERS) {
            this.pendingCalls.delete(packet.streamId)
            const status: Status = decodeStatus(packet.payload)
            if (status.code === StatusCode.OK) {
              pending.resolve(pending.dataBuffer || Buffer.alloc(0))
            } else {
              pending.reject(new RpcError(status.code, status.message))
            }
          }
        }
      } catch (err) {
        if (!this.closed) {
          const detail = err instanceof Error ? err.message : String(err)
          // Reject all pending calls on transport error
          for (const [streamId, pending] of this.pendingCalls) {
            pending.reject(new RpcError(StatusCode.UNAVAILABLE, `Transport error: ${detail}`))
            this.pendingCalls.delete(streamId)
          }
        }
      } finally {
        this.loopRunning = false
        // Restart if new sends were queued during shutdown
        if (this.sendQueue.length > 0 && !this.closed) {
          this.ensureLoop()
        }
      }
    }
    loop().catch(() => {})
  }

  /**
   * Make a raw unary RPC call (no protobuf serialization of request/response payload).
   *
   * Registers a pending call, queues HEADERS + DATA + TRAILERS for sending,
   * then waits for the processing loop to deliver the response. Supports
   * unlimited concurrent calls (each gets a unique stream ID).
   */
  async callRaw(method: string, payload: Buffer, metadata?: Record<string, string>): Promise<Buffer> {
    if (this.closed) {
      throw new RpcError(StatusCode.CANCELLED, 'Peer closed')
    }

    const streamId = this.allocStreamId()
    const call: Call = { method, metadata: metadata || {} }

    // Register pending call BEFORE queuing sends (matching Rust pattern)
    const resultPromise = new Promise<Buffer>((resolve, reject) => {
      this.pendingCalls.set(streamId, { resolve, reject })
    })

    // Queue all sends (processed by the loop without holding recv lock)
    this.sendQueue.push(
      encodePacket(headersPacket(streamId, call)),
      encodePacket(dataPacket(streamId, payload)),
      encodePacket(trailersPacket(streamId, statusOk())),
    )

    // Ensure the processing loop is running
    this.ensureLoop()

    // Apply timeout if :timeout metadata is set
    const timeoutStr = call.metadata[':timeout']
    if (timeoutStr) {
      const timeoutMs = parseInt(timeoutStr, 10)
      if (timeoutMs > 0) {
        const timeoutPromise = new Promise<never>((_, reject) => {
          setTimeout(() => {
            this.pendingCalls.delete(streamId)
            reject(new RpcError(StatusCode.DEADLINE_EXCEEDED, 'Deadline exceeded'))
          }, timeoutMs)
        })
        return Promise.race([resultPromise, timeoutPromise])
      }
    }

    // Wait for response from processing loop
    return resultPromise
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
