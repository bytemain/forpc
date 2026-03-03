/**
 * RawServer - Server-side RPC handler for forpc
 *
 * Listens for incoming RPC calls and dispatches them to registered handlers.
 * Uses AsyncRouter for the underlying transport.
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

export type RawHandler = (payload: Buffer, metadata: Record<string, string>) => Buffer | Promise<Buffer>

interface StreamState {
  method: string
  metadata: Record<string, string>
  routerMsg: RouterMessage
  data?: Buffer
}

/**
 * RawServer - Listens for RPC calls and dispatches to handlers
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
   * Register a handler for a method
   */
  register(method: string, handler: RawHandler): void {
    this.handlers.set(method, handler)
  }

  /**
   * Start serving incoming RPC calls
   */
  serve(): void {
    this.startServeLoop()
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
          const result = await handler(stream.data || Buffer.alloc(0), stream.metadata)

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
