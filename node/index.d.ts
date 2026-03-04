/**
 * forpc - A fast RPC library for Node.js
 *
 * This module re-exports the type declarations from the transport layer,
 * plus the protocol, Peer, and RawServer modules for RPC support.
 */

export * from './transport'
export * from './src/protocol'
export { Peer, RpcError } from './src/peer'
export type { MessageType, ServiceDefinition } from './src/peer'
export { RawServer } from './src/server'
export type { RawHandler, UnaryHandler, ServiceImplementation } from './src/server'

/**
 * Convenience helper to define a service definition with proper typing.
 */
export function defineService(definition: ServiceDefinition): ServiceDefinition
