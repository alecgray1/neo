#!/usr/bin/env node

import { mkdir, writeFile } from "fs/promises";
import { join } from "path";

// Parse args
const args = process.argv.slice(2);
const projectName = args[0];

if (!projectName) {
  console.error("Usage: npm create neo-plugin <project-name>");
  console.error("Example: npm create neo-plugin my-plugin");
  process.exit(1);
}

// Derive plugin ID from project name (kebab-case)
const pluginId = projectName.toLowerCase().replace(/[^a-z0-9]+/g, "-");
const pluginName = projectName
  .split(/[-_\s]+/)
  .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
  .join(" ");

const projectDir = join(process.cwd(), projectName);

console.log(`Creating Neo plugin: ${pluginName} (${pluginId})`);
console.log(`Directory: ${projectDir}\n`);

// Templates
const packageJson = `{
  "name": "${pluginId}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "npm run dev:server & npm run dev:app",
    "dev:server": "vite build --watch",
    "dev:app": "NEO_BUILD_TARGET=app vite build --watch",
    "build": "npm run build:server && npm run build:app",
    "build:server": "vite build",
    "build:app": "NEO_BUILD_TARGET=app vite build",
    "typecheck": "tsc --noEmit"
  },
  "devDependencies": {
    "@neo/vite-plugin": "file:/home/alec/Work/neo/packages/vite-plugin",
    "typescript": "^5.0.0",
    "vite": "^6.0.0"
  }
}
`;

const viteConfig = `import { defineConfig } from "vite";
import neo from "@neo/vite-plugin";

export default defineConfig({
  plugins: [
    neo({
      id: "${pluginId}",
      name: "${pluginName}",

      // Server-side plugin config (runs in Neo's runtime)
      server: {
        // subscriptions: ["temperature/*"],
        // tickInterval: 1000,
      },

      // App-side extension config (runs in Neo app)
      app: {
        activationEvents: ["onStartupFinished"],
        contributes: {
          commands: [
            { id: "${pluginId}.helloWorld", title: "Hello World", category: "${pluginName}" }
          ],
        },
      },
    }),
  ],
});
`;

const tsconfig = `{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "skipLibCheck": true,
    "noEmit": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "lib": ["ES2022", "DOM"]
  },
  "include": ["src/**/*", "neo.d.ts", "app.d.ts"]
}
`;

const neoDts = `/**
 * Neo Plugin API Type Definitions
 */

/** Node input/output pin definition */
interface PinDef {
  name: string;
  type: string;
  description?: string;
}

/** Node definition for blueprint nodes */
interface NodeDef {
  /** Unique node ID (e.g., "myPlugin/Add") */
  id: string;
  /** Display name */
  name: string;
  /** Category for grouping in the node palette */
  category?: string;
  /** Node description */
  description?: string;
  /** Input pins */
  inputs?: PinDef[];
  /** Output pins */
  outputs?: PinDef[];
  /** Pure nodes have no side effects */
  pure?: boolean;
  /** Latent nodes are async */
  latent?: boolean;
  /** Execute function */
  execute: (ctx: NodeContext) => Promise<Record<string, unknown>>;
}

/** Context passed to node execute function */
interface NodeContext {
  nodeId: string;
  config: Record<string, unknown>;
  inputs: Record<string, unknown>;
  variables: Record<string, unknown>;
  getInput(name: string): unknown;
  getConfig(key: string): unknown;
  getVariable(name: string): unknown;
}

/** Service definition */
interface ServiceDef {
  id: string;
  name: string;
  onStart?: () => Promise<void>;
  onStop?: () => Promise<void>;
  onTick?: () => Promise<void>;
  onEvent?: (event: NeoEvent) => Promise<void>;
}

/** Event object */
interface NeoEvent {
  type: string;
  source: string;
  data?: unknown;
  timestamp?: number;
}

/** Type definition */
interface TypeDef {
  id: string;
  name: string;
  description?: string;
  schema?: unknown;
}

declare global {
  const Neo: {
    log: {
      trace(message: string): void;
      debug(message: string): void;
      info(message: string): void;
      warn(message: string): void;
      error(message: string): void;
    };

    points: {
      read(pointId: string): Promise<unknown>;
      write(pointId: string, value: unknown): Promise<void>;
    };

    events: {
      emit(type: string, data?: unknown): void;
    };

    nodes: {
      register(def: NodeDef): void;
      get(id: string): NodeDef | undefined;
      list(): string[];
    };

    services: {
      register(def: ServiceDef): void;
      get(id: string): ServiceDef | undefined;
      list(): string[];
    };

    types: {
      register(def: TypeDef): void;
      get(id: string): TypeDef | undefined;
      list(): string[];
    };

    utils: {
      now(): number;
    };
  };
}

export {};
`;

// App-side type definitions (for neo.* API in extension host)
const appDts = `/**
 * Neo App Extension API Type Definitions
 *
 * These types are available in src/app/ code that runs in the Neo app's extension host.
 */

/** Disposable interface for cleanup */
interface Disposable {
  dispose(): void;
}

/** Extension context passed to activate() */
interface ExtensionContext {
  readonly extensionPath: string;
  readonly subscriptions: Disposable[];
}

/** Quick pick item */
interface QuickPickItem {
  label: string;
  description?: string;
  detail?: string;
}

declare global {
  /** Neo app extension API */
  const neo: {
    readonly version: string;

    /** Command registration and execution */
    commands: {
      registerCommand(id: string, handler: (...args: unknown[]) => unknown): Disposable;
      executeCommand<T = unknown>(id: string, ...args: unknown[]): Promise<T>;
    };

    /** Window/UI operations */
    window: {
      showInformationMessage(message: string, ...items: string[]): Promise<string | undefined>;
      showWarningMessage(message: string, ...items: string[]): Promise<string | undefined>;
      showErrorMessage(message: string, ...items: string[]): Promise<string | undefined>;
      showQuickPick(items: QuickPickItem[] | string[], options?: { placeHolder?: string }): Promise<QuickPickItem | string | undefined>;
      showInputBox(options?: { prompt?: string; placeHolder?: string; value?: string }): Promise<string | undefined>;
    };

    /** Context key management */
    context: {
      set(key: string, value: unknown): void;
    };

    /** Server communication */
    server: {
      readonly connected: boolean;
      request<T = unknown>(path: string, params?: Record<string, unknown>): Promise<T>;
      subscribe(paths: string[]): Promise<Disposable>;
      onDidReceiveChange(listener: (event: { path: string; type: string; value?: unknown }) => void): Disposable;
    };
  };
}

export type { ExtensionContext, Disposable, QuickPickItem };
`;

const indexTs = `/**
 * ${pluginName}
 *
 * A Neo plugin that registers nodes and services.
 */

// Register a background service
Neo.services.register({
  id: "${pluginId}/main",
  name: "${pluginName} Service",

  onStart: async () => {
    Neo.log.info("${pluginName} service started!");
  },

  onStop: async () => {
    Neo.log.info("${pluginName} service stopped");
  },

  onTick: async () => {
    // Called periodically if tick_interval is configured
  },
});

// Register a blueprint node
Neo.nodes.register({
  id: "${pluginId}/Example",
  name: "Example Node",
  category: "${pluginName}",
  description: "An example node that doubles a number",
  inputs: [
    { name: "value", type: "number" },
  ],
  outputs: [
    { name: "result", type: "number" },
  ],
  pure: true,

  execute: async (ctx) => {
    const value = (ctx.getInput("value") as number) || 0;
    return { result: value * 2 };
  },
});

Neo.log.info("${pluginName} plugin loaded");
`;

const gitignore = `node_modules/
dist/
`;

// App extension template (for when app config is enabled)
const appIndexTs = `/**
 * ${pluginName} - App Extension
 *
 * This file runs in the Neo app's extension host.
 * It has access to the \`neo\` API for UI and commands.
 */

import type { ExtensionContext } from "../../app.d.ts";

// Called when the extension is activated
export function activate(context: ExtensionContext) {
  console.log("${pluginName} extension activated!");

  // Register a command
  const disposable = neo.commands.registerCommand("${pluginId}.helloWorld", () => {
    neo.window.showInformationMessage("Hello from ${pluginName}!");
  });

  context.subscriptions.push(disposable);
}

// Called when the extension is deactivated
export function deactivate() {
  console.log("${pluginName} extension deactivated");
}
`;

// Create project
try {
  await mkdir(projectDir);
  await mkdir(join(projectDir, "src"));
  await mkdir(join(projectDir, "src", "server"));
  await mkdir(join(projectDir, "src", "app"));

  await writeFile(join(projectDir, "package.json"), packageJson);
  await writeFile(join(projectDir, "vite.config.ts"), viteConfig);
  await writeFile(join(projectDir, "tsconfig.json"), tsconfig);
  await writeFile(join(projectDir, "neo.d.ts"), neoDts);
  await writeFile(join(projectDir, "app.d.ts"), appDts);
  await writeFile(join(projectDir, "src", "server", "index.ts"), indexTs);
  await writeFile(join(projectDir, "src", "app", "index.ts"), appIndexTs);
  await writeFile(join(projectDir, ".gitignore"), gitignore);

  console.log("Created files:");
  console.log("  package.json");
  console.log("  vite.config.ts");
  console.log("  tsconfig.json");
  console.log("  neo.d.ts            - Server-side type definitions");
  console.log("  app.d.ts            - App-side type definitions");
  console.log("  src/server/index.ts - Server-side plugin (Neo runtime)");
  console.log("  src/app/index.ts    - App extension (Neo UI)");
  console.log("  .gitignore");
  console.log("");
  console.log("Next steps:");
  console.log(`  cd ${projectName}`);
  console.log("  npm install");
  console.log("  npm run dev    # Watch both server & app");
  console.log("  npm run build  # Build both server & app");
} catch (err) {
  if (err.code === "EEXIST") {
    console.error(`Error: Directory '${projectName}' already exists`);
  } else {
    console.error("Error:", err.message);
  }
  process.exit(1);
}
