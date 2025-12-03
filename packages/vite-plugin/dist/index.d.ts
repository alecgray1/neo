import { Plugin } from "vite";
/**
 * Service entry in the manifest
 */
export interface ServiceEntry {
    /** Full service ID: "plugin-id/service-name" */
    id: string;
    /** Path to the built chunk relative to dist */
    entry: string;
    /** Tick interval in milliseconds (extracted from defineService) */
    tickInterval?: number;
    /** Event subscriptions (extracted from defineService) */
    subscriptions?: string[];
}
/**
 * Node entry in the manifest
 */
export interface NodeEntry {
    /** Full node ID: "plugin-id/node-name" */
    id: string;
    /** Path to the built chunk relative to dist */
    entry: string;
}
/**
 * Neo plugin manifest (written to neo-plugin.json)
 */
export interface NeoPluginManifest {
    id: string;
    name: string;
    description?: string;
    services: ServiceEntry[];
    nodes: NodeEntry[];
}
/**
 * Plugin configuration
 */
export interface NeoPluginConfig {
    /** Unique plugin identifier */
    id: string;
    /** Human-readable name */
    name: string;
    /** Description */
    description?: string;
    /** Neo dev server URL for hot reload (default: ws://localhost:9600/ws) */
    devServer?: string;
}
/**
 * Create a Vite plugin for Neo plugin development
 *
 * This plugin:
 * 1. Scans src/services/*.ts and src/nodes/*.ts for service/node definitions
 * 2. Builds each as a separate chunk (one service/node per file)
 * 3. Generates a manifest (neo-plugin.json) with all entries
 * 4. Supports hot reload via WebSocket to Neo dev server
 */
export declare function neo(options: NeoPluginConfig): Plugin;
export default neo;
//# sourceMappingURL=index.d.ts.map