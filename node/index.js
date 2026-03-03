/**
 * forpc - A fast RPC library for Node.js
 *
 * This module re-exports the native bindings from the transport layer,
 * plus the protocol, Peer, and RawServer modules for RPC support.
 */

let transport
try {
  transport = require('./transport')
} catch (e) {
  // Print the cause chain so that native binding load errors
  // (e.g. glibc version mismatch) are visible in CI logs,
  // since most runtimes don't print Error.cause by default.
  let current = e
  while (current) {
    console.error(current.message)
    current = current.cause
  }
  throw e
}

// Re-export all native bindings
module.exports = transport
module.exports.AsyncDealer = transport.AsyncDealer
module.exports.AsyncRouter = transport.AsyncRouter
module.exports.DealerMessage = transport.DealerMessage
module.exports.RouterMessage = transport.RouterMessage
module.exports.plus100 = transport.plus100

// Re-export protocol types and helpers
const protocol = require('./src/protocol')
module.exports.FrameKind = protocol.FrameKind
module.exports.StatusCode = protocol.StatusCode
module.exports.encodeCall = protocol.encodeCall
module.exports.decodeCall = protocol.decodeCall
module.exports.encodeStatus = protocol.encodeStatus
module.exports.decodeStatus = protocol.decodeStatus
module.exports.encodePacket = protocol.encodePacket
module.exports.decodePacket = protocol.decodePacket
module.exports.headersPacket = protocol.headersPacket
module.exports.dataPacket = protocol.dataPacket
module.exports.trailersPacket = protocol.trailersPacket
module.exports.statusOk = protocol.statusOk

// Re-export RPC classes
const { Peer, RpcError } = require('./src/peer')
module.exports.Peer = Peer
module.exports.RpcError = RpcError

const { RawServer } = require('./src/server')
module.exports.RawServer = RawServer
