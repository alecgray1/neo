import { Plugin, ResolvedConfig } from "vite";
import { resolve, join } from "path";
import { writeFileSync, mkdirSync, existsSync, readFileSync } from "fs";
import WebSocket from "ws";

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

  // Legacy fields for backwards compatibility
  /** @deprecated Use server.subscriptions instead */
  subscriptions?: string[];
  /** @deprecated Use server.tickInterval instead */
  tickInterval?: number;
  /** @deprecated Use server.config instead */
  config?: Record<string, unknown>;
}

/**
 * Unified plugin manifest (written to package.json neo field)
 */
interface NeoPluginManifest {
  id: string;
  name: string;
  description?: string;
  server?: {
    entry: string;
    subscriptions: string[];
    tickInterval?: number;
    config: Record<string, unknown>;
  };
  app?: {
    entry: string;
    activationEvents?: string[];
    contributes?: AppConfig["contributes"];
  };
}

/**
 * Legacy manifest format for backwards compatibility
 */
interface LegacyManifest {
  id: string;
  name: string;
  description?: string;
  entry: string;
  subscriptions: string[];
  tickInterval?: number;
  config: Record<string, unknown>;
}

/**
 * Detect which targets are available based on file structure
 */
function detectBuildTargets(pluginDir: string, options: NeoPluginConfig): BuildTarget[] {
  const targets: BuildTarget[] = [];

  // Check for server code
  const serverEntry = options.server?.entry ?? "src/server/index.ts";
  const legacyEntry = "src/index.ts";

  if (existsSync(resolve(pluginDir, serverEntry))) {
    targets.push("server");
  } else if (existsSync(resolve(pluginDir, legacyEntry)) && !options.app) {
    // Legacy mode: single entry point for server
    targets.push("server");
  }

  // Check for app code
  const appEntry = options.app?.entry ?? "src/app/index.ts";
  if (existsSync(resolve(pluginDir, appEntry))) {
    targets.push("app");
  }

  return targets;
}

/**
 * Create a Vite plugin for Neo plugin development
 */
export function neo(options: NeoPluginConfig): Plugin {
  let resolvedConfig: ResolvedConfig;
  let ws: WebSocket | null = null;
  let isDevMode = false;
  let outputPath = "";
  let pluginDir = "";
  let buildTarget: BuildTarget = "server"; // Default to server for legacy compatibility

  const devServerUrl = options.devServer ?? "ws://localhost:9600/ws";
  let isRegistered = false;

  // Normalize legacy options to new format
  const normalizedOptions: NeoPluginConfig = {
    ...options,
    server: options.server ?? (options.subscriptions || options.tickInterval || options.config
      ? {
          subscriptions: options.subscriptions,
          tickInterval: options.tickInterval,
          config: options.config,
        }
      : undefined),
  };

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

  const registerPlugin = () => {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;

    const serverConfig = normalizedOptions.server;
    const msg = {
      type: "plugin:register",
      plugin: {
        id: options.id,
        name: options.name,
        description: options.description,
        entryPath: outputPath,
        subscriptions: serverConfig?.subscriptions ?? [],
        tickInterval: serverConfig?.tickInterval,
        config: serverConfig?.config ?? {},
      },
    };

    ws.send(JSON.stringify(msg));
  };

  const notifyRebuilt = () => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
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

  /**
   * Write the unified manifest (package.json with neo field)
   */
  const writeManifest = (outDir: string, target: BuildTarget) => {
    // For the new unified format, we update package.json with neo field
    const packageJsonPath = join(pluginDir, "package.json");
    let packageJson: Record<string, unknown> = {};

    try {
      const content = readFileSync(packageJsonPath, "utf-8");
      packageJson = JSON.parse(content);
    } catch {
      // No package.json, create minimal one
      packageJson = {
        name: options.id,
        version: "0.1.0",
      };
    }

    // Build the neo manifest section
    const neoManifest: NeoPluginManifest = {
      id: options.id,
      name: options.name,
      description: options.description,
    };

    // Add server config if building server target
    if (target === "server" || normalizedOptions.server) {
      neoManifest.server = {
        entry: `dist/server/index.js`,
        subscriptions: normalizedOptions.server?.subscriptions ?? [],
        tickInterval: normalizedOptions.server?.tickInterval,
        config: normalizedOptions.server?.config ?? {},
      };
    }

    // Add app config if configured
    if (options.app) {
      neoManifest.app = {
        entry: `dist/app/index.js`,
        activationEvents: options.app.activationEvents,
        contributes: options.app.contributes,
      };
    }

    // Update package.json with neo field
    packageJson.neo = neoManifest;

    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2));
    console.log(`[neo] Updated package.json with neo manifest`);

    // Also write legacy neo-plugin.json for backwards compatibility
    if (target === "server") {
      const legacyManifest: LegacyManifest = {
        id: options.id,
        name: options.name,
        description: options.description,
        entry: "index.js",
        subscriptions: normalizedOptions.server?.subscriptions ?? [],
        tickInterval: normalizedOptions.server?.tickInterval,
        config: normalizedOptions.server?.config ?? {},
      };

      const manifestPath = join(outDir, "neo-plugin.json");
      writeFileSync(manifestPath, JSON.stringify(legacyManifest, null, 2));
      console.log(`[neo] Wrote legacy manifest: ${manifestPath}`);
    }
  };

  return {
    name: "neo-plugin",

    config(config, { command }) {
      isDevMode = command === "serve" || process.argv.includes("--watch");
      pluginDir = process.cwd();

      // Determine build target from env or auto-detect
      const envTarget = process.env.NEO_BUILD_TARGET as BuildTarget | undefined;
      const availableTargets = detectBuildTargets(pluginDir, options);

      if (envTarget && availableTargets.includes(envTarget)) {
        buildTarget = envTarget;
      } else if (availableTargets.length > 0) {
        // Default to first available target (server has priority for legacy compatibility)
        buildTarget = availableTargets.includes("server") ? "server" : availableTargets[0];
      }

      console.log(`[neo] Building ${buildTarget} target for plugin: ${options.id}`);

      // Determine entry point based on target
      let entryPath: string;
      if (buildTarget === "app") {
        entryPath = options.app?.entry ?? "src/app/index.ts";
      } else {
        // Server target
        entryPath = normalizedOptions.server?.entry ?? "src/server/index.ts";
        // Fall back to legacy entry if new structure doesn't exist
        if (!existsSync(resolve(pluginDir, entryPath))) {
          entryPath = "src/index.ts";
        }
      }

      // Determine output path
      const neoProject = process.env.NEO_PROJECT;
      let outDir: string;

      if (!isDevMode && neoProject) {
        // Production build: output to Neo project
        if (buildTarget === "app") {
          // App code goes to extensions directory
          outDir = join(neoProject, "extensions", options.id, "dist", "app");
        } else {
          // Server code goes to plugins directory
          outDir = join(neoProject, "plugins", options.id, "dist");
        }
        mkdirSync(outDir, { recursive: true });
      } else {
        // Dev build: output locally with target subdirectory
        outDir = buildTarget === "app" ? "dist/app" : "dist/server";

        // For legacy plugins without the new structure, use just "dist"
        if (buildTarget === "server" && !existsSync(resolve(pluginDir, "src/server"))) {
          outDir = "dist";
        }
      }

      outputPath = resolve(pluginDir, outDir, "index.js");

      return {
        build: {
          lib: {
            entry: resolve(pluginDir, entryPath),
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
            // For app builds, externalize neo extension API
            external: buildTarget === "app" ? [/^@anthropic\/neo-extension-api/] : [],
          },
        },
      };
    },

    configResolved(config) {
      resolvedConfig = config;
    },

    buildStart() {
      // Only connect to Neo server for server builds
      if (isDevMode && buildTarget === "server") {
        connectToNeo();
      }
    },

    writeBundle() {
      const outDir = resolvedConfig.build.outDir;

      // Write manifest
      writeManifest(resolve(pluginDir, outDir), buildTarget);

      // Only register/notify for server builds
      if (isDevMode && buildTarget === "server") {
        if (!ws) {
          connectToNeo();
        }

        setTimeout(() => {
          if (!isRegistered) {
            registerPlugin();
          } else {
            notifyRebuilt();
          }
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

/**
 * Create plugins for building both server and app targets
 * Use this when you want to build both in a single vite config
 */
export function neoDual(options: NeoPluginConfig): Plugin[] {
  const plugins: Plugin[] = [];

  // Create server plugin if server config exists
  if (options.server || options.subscriptions) {
    plugins.push({
      ...neo({ ...options, app: undefined }),
      name: "neo-plugin-server",
    });
  }

  // Create app plugin if app config exists
  if (options.app) {
    plugins.push({
      ...neo({ ...options, server: undefined }),
      name: "neo-plugin-app",
    });
  }

  return plugins;
}

export default neo;
