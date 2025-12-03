import { Plugin, ResolvedConfig } from "vite";
import { resolve, join, basename, extname } from "path";
import { writeFileSync, mkdirSync, existsSync, readFileSync, readdirSync } from "fs";
import { glob } from "glob";
import WebSocket from "ws";

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
 * Scan for service and node files in the plugin directory
 */
async function scanPluginFiles(pluginDir: string): Promise<{
  services: string[];
  nodes: string[];
}> {
  const srcDir = join(pluginDir, "src");

  // Find all service files
  const servicePatterns = [
    join(srcDir, "services", "*.ts"),
    join(srcDir, "services", "*.js"),
  ];

  // Find all node files
  const nodePatterns = [
    join(srcDir, "nodes", "*.ts"),
    join(srcDir, "nodes", "*.js"),
  ];

  const services: string[] = [];
  const nodes: string[] = [];

  for (const pattern of servicePatterns) {
    const matches = await glob(pattern.replace(/\\/g, "/"));
    services.push(...matches);
  }

  for (const pattern of nodePatterns) {
    const matches = await glob(pattern.replace(/\\/g, "/"));
    nodes.push(...matches);
  }

  return { services, nodes };
}

/**
 * Extract config from a built service chunk
 * Looks for tickInterval and subscriptions in the defineService call
 */
function extractServiceConfig(code: string): { tickInterval?: number; subscriptions?: string[] } {
  const config: { tickInterval?: number; subscriptions?: string[] } = {};

  // Match tickInterval: <number> - handles regular numbers, scientific notation (5e3), and underscores (5_000)
  const tickMatch = code.match(/tickInterval\s*:\s*([\d._]+(?:e\d+)?)/i);
  if (tickMatch) {
    // Remove underscores and parse (handles 5_000, 5e3, etc.)
    config.tickInterval = Number(tickMatch[1].replace(/_/g, ""));
  }

  // Match subscriptions: ["...", "..."]
  const subsMatch = code.match(/subscriptions\s*:\s*\[([^\]]*)\]/);
  if (subsMatch) {
    const subsContent = subsMatch[1];
    const strings = subsContent.match(/"([^"]+)"|'([^']+)'/g);
    if (strings) {
      config.subscriptions = strings.map(s => s.slice(1, -1));
    }
  }

  return config;
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
export function neo(options: NeoPluginConfig): Plugin {
  let resolvedConfig: ResolvedConfig;
  let ws: WebSocket | null = null;
  let isDevMode = false;
  let pluginDir = "";
  let serviceFiles: string[] = [];
  let nodeFiles: string[] = [];

  const devServerUrl = options.devServer ?? "ws://localhost:9600/ws";
  let isRegistered = false;

  const connectToNeo = () => {
    if (ws) return;

    ws = new WebSocket(devServerUrl);

    ws.on("open", () => {
      console.log(`[neo] Connected to Neo server`);
    });

    ws.on("error", (err) => {
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

  const notifyRebuilt = (manifest: NeoPluginManifest) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      connectToNeo();
      return;
    }

    const msg = {
      type: "plugin:rebuilt",
      pluginId: options.id,
      manifest,
    };

    ws.send(JSON.stringify(msg));
    console.log(`[neo] Notified Neo of rebuild`);
  };

  return {
    name: "neo-plugin",

    async config(config, { command }) {
      isDevMode = command === "serve" || process.argv.includes("--watch");
      pluginDir = process.cwd();

      // Scan for service and node files
      const files = await scanPluginFiles(pluginDir);
      serviceFiles = files.services;
      nodeFiles = files.nodes;

      console.log(`[neo] Building plugin: ${options.id}`);
      console.log(`[neo] Found ${serviceFiles.length} services, ${nodeFiles.length} nodes`);

      // Build input entries for each service and node
      const input: Record<string, string> = {};

      for (const file of serviceFiles) {
        const name = basename(file, extname(file));
        input[`services/${name}`] = file;
      }

      for (const file of nodeFiles) {
        const name = basename(file, extname(file));
        input[`nodes/${name}`] = file;
      }

      // If no services or nodes found, check for legacy single entry
      if (Object.keys(input).length === 0) {
        const legacyEntry = join(pluginDir, "src", "index.ts");
        if (existsSync(legacyEntry)) {
          console.log(`[neo] Warning: Using legacy single-file mode. Consider migrating to src/services/*.ts`);
          input["index"] = legacyEntry;
        }
      }

      return {
        build: {
          outDir: "dist",
          emptyOutDir: true,
          minify: isDevMode ? false : "esbuild",
          rollupOptions: {
            input,
            output: {
              dir: "dist",
              format: "es",
              entryFileNames: "[name].js",
              chunkFileNames: "chunks/[name]-[hash].js",
              // Don't inline dynamic imports - we want separate chunks
              inlineDynamicImports: false,
            },
            // Don't externalize anything - each chunk should be self-contained
            external: [],
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

    async writeBundle(outputOptions, bundle) {
      const outDir = resolve(pluginDir, "dist");

      // Build the manifest
      const manifest: NeoPluginManifest = {
        id: options.id,
        name: options.name,
        description: options.description,
        services: [],
        nodes: [],
      };

      // Process each output chunk
      for (const [fileName, chunk] of Object.entries(bundle)) {
        if (chunk.type !== "chunk" || !chunk.isEntry) continue;

        const name = basename(fileName, ".js");
        const dir = fileName.split("/")[0];

        if (dir === "services") {
          // Extract config from the built code
          const code = chunk.code;
          const config = extractServiceConfig(code);

          manifest.services.push({
            id: `${options.id}/${name}`,
            entry: fileName,
            tickInterval: config.tickInterval,
            subscriptions: config.subscriptions,
          });
        } else if (dir === "nodes") {
          manifest.nodes.push({
            id: `${options.id}/${name}`,
            entry: fileName,
          });
        }
      }

      // Write manifest
      const manifestPath = join(outDir, "neo-plugin.json");
      writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
      console.log(`[neo] Wrote manifest: ${manifestPath}`);
      console.log(`[neo]   - ${manifest.services.length} services`);
      console.log(`[neo]   - ${manifest.nodes.length} nodes`);

      // Also update package.json with neo field
      const packageJsonPath = join(pluginDir, "package.json");
      try {
        let packageJson: Record<string, unknown> = {};
        if (existsSync(packageJsonPath)) {
          packageJson = JSON.parse(readFileSync(packageJsonPath, "utf-8"));
        }
        packageJson.neo = manifest;
        writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
      } catch (e) {
        console.warn(`[neo] Failed to update package.json:`, e);
      }

      // Notify Neo dev server
      if (isDevMode) {
        setTimeout(() => {
          notifyRebuilt(manifest);
        }, 100);
      }
    },

    closeBundle() {
      if (!isDevMode && ws) {
        ws.close();
        ws = null;
      }
    },
  };
}

export default neo;
