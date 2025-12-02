import { Plugin, ResolvedConfig } from "vite";
import { resolve, join } from "path";
import { writeFileSync, mkdirSync } from "fs";
import WebSocket from "ws";

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

interface NeoPluginManifest {
  id: string;
  name: string;
  description?: string;
  entry: string;
  subscriptions: string[];
  tickInterval?: number;
  config: Record<string, unknown>;
}

export function neo(options: NeoPluginConfig): Plugin {
  let resolvedConfig: ResolvedConfig;
  let ws: WebSocket | null = null;
  let isDevMode = false;
  let outputPath = "";
  let pluginDir = "";

  const devServerUrl = options.devServer ?? "ws://localhost:9600/ws";

  let isRegistered = false;

  const connectToNeo = () => {
    if (ws) return;

    ws = new WebSocket(devServerUrl);

    ws.on("open", () => {
      console.log(`[neo] Connected to Neo server`);
      // Don't register here - wait for first build to complete
    });

    ws.on("error", (err) => {
      // Neo server might not be running yet, that's okay
      if ((err as NodeJS.ErrnoException).code === "ECONNREFUSED") {
        console.log(`[neo] Neo server not running, will retry on rebuild...`);
      } else {
        console.error(`[neo] WebSocket error:`, err.message);
      }
      ws = null;
    });

    ws.on("close", () => {
      console.log(`[neo] Disconnected from Neo server`);
      ws = null;
      isRegistered = false;
    });

    ws.on("message", (data) => {
      try {
        const msg = JSON.parse(data.toString());
        if (msg.type === "plugin:registered") {
          console.log(`[neo] Plugin registered: ${options.id}`);
          isRegistered = true;
        } else if (msg.type === "plugin:restarted") {
          console.log(`[neo] Plugin restarted: ${options.id}`);
        }
      } catch {
        // Ignore parse errors
      }
    });
  };

  const registerPlugin = () => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;

    const msg = {
      type: "plugin:register",
      plugin: {
        id: options.id,
        name: options.name,
        description: options.description,
        entryPath: outputPath,
        subscriptions: options.subscriptions ?? [],
        tickInterval: options.tickInterval,
        config: options.config ?? {},
      },
    };

    ws.send(JSON.stringify(msg));
  };

  const notifyRebuilt = () => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      // Try to connect if not connected
      connectToNeo();
      return;
    }

    const msg = {
      type: "plugin:rebuilt",
      pluginId: options.id,
      entryPath: outputPath,
    };

    ws.send(JSON.stringify(msg));
    console.log(`[neo] Notified Neo of rebuild`);
  };

  const writeManifest = (outDir: string) => {
    const manifest: NeoPluginManifest = {
      id: options.id,
      name: options.name,
      description: options.description,
      entry: "index.js",
      subscriptions: options.subscriptions ?? [],
      tickInterval: options.tickInterval,
      config: options.config ?? {},
    };

    const manifestPath = join(outDir, "neo-plugin.json");
    writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
    console.log(`[neo] Wrote manifest: ${manifestPath}`);
  };

  return {
    name: "neo-plugin",

    config(config, { command }) {
      isDevMode = command === "serve" || process.argv.includes("--watch");
      pluginDir = process.cwd();

      // Determine output path
      const neoProject = process.env.NEO_PROJECT;
      let outDir: string;

      if (!isDevMode && neoProject) {
        // Production build: output to Neo project plugins directory
        outDir = join(neoProject, "plugins", options.id, "dist");
        mkdirSync(outDir, { recursive: true });
      } else {
        // Dev build: output locally
        outDir = "dist";
      }

      outputPath = resolve(pluginDir, outDir, "index.js");

      return {
        build: {
          lib: {
            entry: resolve(pluginDir, "src/index.ts"),
            formats: ["es"] as const,
            fileName: () => "index.js",
          },
          minify: isDevMode ? false : "esbuild",
          outDir,
          emptyOutDir: true,
          rollupOptions: {
            output: {
              inlineDynamicImports: true,
            },
          },
        },
      };
    },

    configResolved(config) {
      resolvedConfig = config;
    },

    buildStart() {
      if (isDevMode) {
        connectToNeo();
      }
    },

    writeBundle() {
      const outDir = resolvedConfig.build.outDir;

      // Always write manifest
      writeManifest(resolve(pluginDir, outDir));

      if (isDevMode) {
        // Connect if not connected
        if (!ws) {
          connectToNeo();
        }

        // Wait a tick for WebSocket to be ready, then register or notify
        setTimeout(() => {
          if (!isRegistered) {
            // First build - register the plugin
            registerPlugin();
          } else {
            // Subsequent build - notify rebuild
            notifyRebuilt();
          }
        }, 100);
      }
    },

    closeBundle() {
      // Clean up WebSocket on final close (not during watch rebuilds)
      if (!isDevMode && ws) {
        ws.close();
        ws = null;
      }
    },
  };
}

export default neo;
