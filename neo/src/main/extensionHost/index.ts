/**
 * Extension Host Module
 *
 * Exports the extension host system for use by the main process.
 */

export { ExtensionHostMain, getExtensionHostMain } from './ExtensionHostMain'
export { ExtensionScanner } from './ExtensionScanner'
export type { ScannedExtension, ExtensionManifest, ExtensionContributes } from './ExtensionScanner'
export { RPCProtocol, createProxyIdentifier } from './protocol'
export type { ProxyIdentifier, Proxied, RPCMessage } from './protocol'
export * from './extHost.protocol'
