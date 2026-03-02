/**
 * Peer - RPC client for forpc
 *
 * Wraps the low-level AsyncDealer transport with the forpc
 * RPC protocol (Packet framing + protobuf Call/Status messages).
 *
 * Enables Node.js to invoke RPC methods on Rust/Go servers and vice versa.
 */

import { AsyncDealer } from '../transport'
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
} from './protocol'

export class RpcError extends Error {
  code: number

  constructor(code: number, message: string) {
    super(message)
    this.code = code
    this.name = 'RpcError'
  }
}

/**
 * Peer - Client-side RPC peer
 *
 * Connects to a remote RPC server and can invoke unary calls.
 * Uses AsyncDealer for the underlying transport.
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
}
