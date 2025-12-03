import { Plugin } from "vite";
/**
 * Build target for the plugin
 */
export type BuildTarget = "server" | "app";
/**
 * Server-side plugin configuration (runs in Neo's deno_core runtime)
 */
export interface ServerConfig {
    /** Entry point relative to plugin root (default: src/server/index.ts) */
    entry?: string;
    /** Event subscriptions (glob patterns) */
    subscriptions?: string[];
    /** Tick interval in milliseconds */
    tickInterval?: number;
    /** Plugin-specific configuration passed to onStart */
    config?: Record<string, unknown>;
}
/**
 * App-side plugin configuration (runs in Electron extension host)
 */
export interface AppConfig {
    /** Entry point relative to plugin root (default: src/app/index.ts) */
    entry?: string;
    /** Activation events (e.g., 'onStartupFinished', 'onCommand:myCommand') */
    activationEvents?: string[];
    /** Contribution points */
    contributes?: {
        commands?: CommandContribution[];
        viewsContainers?: {
            activitybar?: ViewContainerContribution[];
            panel?: ViewContainerContribution[];
        };
        views?: Record<string, ViewContribution[]>;
        menus?: Record<string, MenuContribution[]>;
        keybindings?: KeybindingContribution[];
    };
}
export interface CommandContribution {
    id: string;
    title: string;
    category?: string;
    icon?: string;
    enablement?: string;
}
export interface ViewContainerContribution {
    id: string;
    title: string;
    icon: string;
}
export interface ViewContribution {
    id: string;
    name: string;
    type?: "tree" | "webview";
    when?: string;
    icon?: string;
}
export interface MenuContribution {
    command: string;
    when?: string;
    group?: string;
}
export interface KeybindingContribution {
    command: string;
    key: string;
    mac?: string;
    when?: string;
}
export interface NeoPluginConfig {
    /** Unique plugin identifier */
    id: string;
    /** Human-readable name */
    name: string;
    /** Description */
    description?: string;
    /** Server-side configuration (optional) */
    server?: ServerConfig;
    /** App-side configuration (optional) */
    app?: AppConfig;
    /** Neo dev server URL for hot reload (default: ws://localhost:9600/ws) */
    devServer?: string;
    /** Electron extension dev server URL for app hot reload (default: ws://localhost:9601) */
    electronDevServer?: string;
    /** @deprecated Use server.subscriptions instead */
    subscriptions?: string[];
    /** @deprecated Use server.tickInterval instead */
    tickInterval?: number;
    /** @deprecated Use server.config instead */
    config?: Record<string, unknown>;
}
/**
 * Create a Vite plugin for Neo plugin development
 */
export declare function neo(options: NeoPluginConfig): Plugin;
/**
 * Create plugins for building both server and app targets
 * Use this when you want to build both in a single vite config
 */
export declare function neoDual(options: NeoPluginConfig): Plugin[];
export default neo;
//# sourceMappingURL=index.d.ts.map