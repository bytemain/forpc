/**
 * RawServer - Server-side RPC handler for forpc
 *
 * Listens for incoming RPC calls and dispatches them to registered handlers.
 * Uses AsyncRouter for the underlying transport.
 *
 * Supports two handler registration modes:
 * - **register**: Low-level raw byte handlers (full control)
 * - **registerUnary**: Typed protobuf handlers with auto serialization (like gRPC/bRPC)
 * - **addService**: Register all methods of a service definition at once (gRPC-style)
 */

import { AsyncRouter, RouterMessage } from '../transport/index.js'
import {
  type Status,
  StatusCode,
  FrameKind,
  encodePacket,
  decodePacket,
  dataPacket,
  trailersPacket,
  statusOk,
  decodeCall,
} from './protocol.ts'

import type { MessageType, ServiceDefinition } from './peer.ts'

export type RawHandler = (payload: Buffer, metadata: Record<string, string>, signal: AbortSignal) => Buffer | Promise<Buffer>

/**
 * Typed unary handler for protobuf-based services.
 *
 * Receives a decoded request object and returns a response object,
 * following the same pattern as Rust's `register_unary` and gRPC service handlers.
 */
export type UnaryHandler<Req = object, Resp = object> = (
  request: Req,
  metadata: Record<string, string>,
  signal: AbortSignal,
) => Resp | Promise<Resp>

/**
 * Service implementation mapping method names to handler functions.
 *
 * Used with `addService` for gRPC-style service registration.
 *
 * @example
 * ```ts
 * const impl: ServiceImplementation = {
 *   Echo: (req, metadata) => ({ result: req.data }),
 * }
 * server.addService('MyService', serviceDefinition, impl)
 * ```
 */
export interface ServiceImplementation {
  [methodName: string]: UnaryHandler
}

interface StreamState {
  method: string
  metadata: Record<string, string>
  routerMsg: RouterMessage
  data?: Buffer
  abortController: AbortController
}

/**
 * RawServer - Listens for RPC calls and dispatches to handlers
 *
 * @example
 * ```ts
 * // Raw handler registration
 * const server = await RawServer.bind(url)
 * server.register('Echo/Ping', (payload) => payload)
 * server.serve()
 *
 * // Typed protobuf handler
 * server.registerUnary('Test/Echo', EchoRequest, EchoResponse, (req) => {
 *   return { result: req.data }
 * })
 *
 * // gRPC-style service registration
 * server.addService('Test', serviceDefinition, {
 *   Echo: (req) => ({ result: req.data }),
 * })
 * ```
 */
export class RawServer {
  private router: AsyncRouter
  private handlers: Map<string, RawHandler>
  private running: boolean
  private streams: Map<number, StreamState>

  private constructor(router: AsyncRouter) {
    this.router = router
    this.handlers = new Map()
    this.running = false
    this.streams = new Map()
  }

  /**
   * Create a server listening on the given URL with a single handler for a method.
   * This is a fire-and-forget convenience factory for simple single-method servers.
   * The server starts asynchronously; for full control use `bind()` + `register()` + `serve()`.
   */
  static listen(url: string, method: string, handler: RawHandler): void {
    AsyncRouter.listen(url).then((router) => {
      const server = new RawServer(router)
      server.handlers.set(method, handler)
      server.startServeLoop()
    })
  }

  /**
   * Create a server by listening on the given URL
   */
  static async bind(url: string): Promise<RawServer> {
    const router = await AsyncRouter.listen(url)
    return new RawServer(router)
  }

  /**
   * Register a raw handler for a method
   */
  register(method: string, handler: RawHandler): void {
    this.handlers.set(method, handler)
  }

  /**
   * Register a typed unary handler with automatic protobuf serialization.
   *
   * Mirrors Rust's `register_unary<Req, Resp>()` — the handler receives
   * a decoded request and returns a response object that is automatically
   * serialized back to protobuf bytes.
   *
   * @param method - Full RPC method name (e.g. "Service/Method")
   * @param requestType - Protobuf message type for decoding the request
   * @param responseType - Protobuf message type for encoding the response
   * @param handler - Handler function receiving decoded request, returning response
   *
   * @example
   * ```ts
   * server.registerUnary('Test/Echo', EchoRequest, EchoResponse, (req) => {
   *   return { result: req.data }
   * })
   * ```
   */
  registerUnary<Req extends object, Resp extends object>(
    method: string,
    requestType: MessageType<Req>,
    responseType: MessageType<Resp>,
    handler: UnaryHandler<Req, Resp>,
  ): void {
    this.handlers.set(method, async (payload: Buffer, metadata: Record<string, string>, signal: AbortSignal) => {
      const request = requestType.decode(payload)
      const response = await handler(request, metadata, signal)
      const respMsg = responseType.create(response)
      return Buffer.from(responseType.encode(respMsg).finish())
    })
  }

  /**
   * Register all methods of a service at once (gRPC-style).
   *
   * Each method in the service definition is registered with the corresponding
   * handler from the implementation object. Method names are prefixed with
   * `serviceName/` (e.g. "MyService/Echo").
   *
   * @param serviceName - Service name prefix (e.g. "MyService")
   * @param definition - Service definition mapping method names to protobuf types
   * @param implementation - Handler implementations for each method
   *
   * @example
   * ```ts
   * const echoServiceDef: ServiceDefinition = {
   *   Echo: { requestType: EchoRequest, responseType: EchoResponse },
   * }
   *
   * server.addService('Test', echoServiceDef, {
   *   Echo: (req) => ({ result: req.data }),
   * })
   * ```
   */
  addService(
    serviceName: string,
    definition: ServiceDefinition,
    implementation: ServiceImplementation,
  ): void {
    for (const methodName of Object.keys(definition)) {
      const handler = implementation[methodName]
      if (!handler) continue
      const { requestType, responseType } = definition[methodName]
      this.registerUnary(`${serviceName}/${methodName}`, requestType, responseType, handler)
    }
  }

  /**
   * Start serving incoming RPC calls
   */
  serve(): void {
    this.startServeLoop()
  }

  /**
   * Stop the server and cancel pending operations
   */
  close(): void {
    this.running = false
    this.router.close()
  }

  private startServeLoop(): void {
    if (this.running) return
    this.running = true

    const loop = async () => {
      while (this.running) {
        try {
          const msg = await this.router.recv()
          const body = msg.body()
          if (body.length < 5) continue

          const packet = decodePacket(body)
          if (packet.streamId === 0) continue

          await this.handlePacket(packet, msg)
        } catch (err) {
          if (this.running) {
            this.running = false
            console.error('RawServer serve loop error:', err)
          }
          break
        }
      }
    }

    loop()
  }

  private async handlePacket(
    packet: { streamId: number; kind: number; payload: Buffer },
    routerMsg: RouterMessage,
  ): Promise<void> {
    switch (packet.kind) {
      case FrameKind.HEADERS: {
        const call = decodeCall(packet.payload)
        this.streams.set(packet.streamId, {
          method: call.method,
          metadata: call.metadata,
          routerMsg,
          abortController: new AbortController(),
        })
        break
      }
      case FrameKind.DATA: {
        const stream = this.streams.get(packet.streamId)
        if (stream) {
          stream.data = packet.payload
          // Update routerMsg to latest for routing
          stream.routerMsg = routerMsg
        }
        break
      }
      case FrameKind.RST_STREAM: {
        // Client cancelled the stream — abort the signal and clean up state
        const stream = this.streams.get(packet.streamId)
        if (stream) {
          stream.abortController.abort()
        }
        this.streams.delete(packet.streamId)
        break
      }
      case FrameKind.TRAILERS: {
        const stream = this.streams.get(packet.streamId)
        if (!stream) break
        this.streams.delete(packet.streamId)

        // Use the latest routerMsg for routing responses
        const responseMsg = routerMsg

        const handler = this.handlers.get(stream.method)
        if (!handler) {
          await this.sendError(
            responseMsg,
            packet.streamId,
            StatusCode.UNIMPLEMENTED,
            `Method ${stream.method} not found`,
          )
          break
        }

        try {
          const result = await handler(stream.data || Buffer.alloc(0), stream.metadata, stream.abortController.signal)

          // Send DATA response
          const respData = dataPacket(packet.streamId, result)
          const encodedData = encodePacket(respData)
          const dataResponse = responseMsg.createResponse(encodedData)
          await this.router.send(dataResponse)

          // Send TRAILERS (OK)
          const respTrailers = trailersPacket(packet.streamId, statusOk())
          const encodedTrailers = encodePacket(respTrailers)
          const trailersResponse = responseMsg.createResponse(encodedTrailers)
          await this.router.send(trailersResponse)
        } catch (err) {
          await this.sendError(responseMsg, packet.streamId, StatusCode.INTERNAL, String(err))
        }
        break
      }
    }
  }

  private async sendError(
    routerMsg: RouterMessage,
    streamId: number,
    code: number,
    message: string,
  ): Promise<void> {
    const status: Status = { code, message }
    const pkt = trailersPacket(streamId, status)
    const encoded = encodePacket(pkt)
    const response = routerMsg.createResponse(encoded)
    await this.router.send(response)
  }
}
