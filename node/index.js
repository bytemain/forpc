/**
 * forpc - A fast RPC library for Node.js
 *
 * This module re-exports the native bindings from the transport layer.
 * The native bindings provide efficient async communication using the nng library.
 */

const transport = require('./transport')

// Re-export all native bindings
module.exports = transport
module.exports.AsyncDealer = transport.AsyncDealer
module.exports.AsyncRouter = transport.AsyncRouter
module.exports.DealerMessage = transport.DealerMessage
module.exports.RouterMessage = transport.RouterMessage
module.exports.plus100 = transport.plus100
