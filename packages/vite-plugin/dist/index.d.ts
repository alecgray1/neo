import { Plugin } from "vite";
export interface NeoPluginConfig {
    /** Unique plugin identifier */
    id: string;
    /** Human-readable name */
    name: string;
    /** Description */
    description?: string;
    /** Event subscriptions (glob patterns) */
    subscriptions?: string[];
    /** Tick interval in milliseconds */
    tickInterval?: number;
    /** Plugin-specific configuration passed to onStart */
    config?: Record<string, unknown>;
    /** Neo dev server URL (default: ws://localhost:9600/ws) */
    devServer?: string;
}
export declare function neo(options: NeoPluginConfig): Plugin;
export default neo;
//# sourceMappingURL=index.d.ts.map