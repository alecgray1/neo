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
    "build": "vite build",
    "dev": "vite build --watch",
    "typecheck": "tsc --noEmit"
  },
  "devDependencies": {
    "@neo/vite-plugin": "workspace:*",
    "typescript": "^5.7.0",
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
      description: "A Neo plugin",
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
    "esModuleInterop": true
  },
  "include": ["src/**/*", "neo.d.ts"]
}
`;

const neoDts = `// Neo Plugin Type Definitions

interface ServiceContext {
  state: Record<string, unknown>;
  config: Record<string, unknown>;
}

interface ServiceConfig {
  name: string;
  tickInterval?: number;
  subscriptions?: string[];
  onStart?: (ctx: ServiceContext) => Promise<void>;
  onStop?: (ctx: ServiceContext) => Promise<void>;
  onTick?: (ctx: ServiceContext) => Promise<void>;
  onEvent?: (ctx: ServiceContext, event: NeoEvent) => Promise<void>;
}

interface NeoEvent {
  type: string;
  source: string;
  data: unknown;
  timestamp: number;
}

interface PinDefinition {
  name: string;
  type: string;
}

interface NodeContext {
  nodeId: string;
  config: Record<string, unknown>;
  inputs: Record<string, unknown>;
  variables: Record<string, unknown>;
  getInput: (name: string) => unknown;
  getConfig: (key: string) => unknown;
  getVariable: (name: string) => unknown;
}

interface NodeConfig {
  name: string;
  category?: string;
  description?: string;
  inputs: PinDefinition[];
  outputs: PinDefinition[];
  pure?: boolean;
  latent?: boolean;
  execute: (ctx: NodeContext) => Promise<Record<string, unknown>>;
}

declare function defineService<T extends ServiceConfig>(config: T): T;
declare function defineNode<T extends NodeConfig>(config: T): T;

declare const Neo: {
  log: {
    error: (msg: string) => void;
    warn: (msg: string) => void;
    info: (msg: string) => void;
    debug: (msg: string) => void;
    trace: (msg: string) => void;
  };
  points: {
    read: (id: string) => Promise<unknown>;
    write: (id: string, value: unknown) => Promise<void>;
  };
  events: {
    emit: (type: string, data: unknown) => void;
  };
  utils: {
    now: () => number;
  };
};
`;

const mainServiceTs = `// Main Service - the primary service for this plugin
export default defineService({
  name: "${pluginName} Service",
  tickInterval: 5000, // Tick every 5 seconds

  onStart: async (ctx) => {
    ctx.state.count = 0;
    Neo.log.info("${pluginName} service started!");
  },

  onStop: async (ctx) => {
    Neo.log.info("${pluginName} service stopped after " + ctx.state.count + " ticks");
  },

  onTick: async (ctx) => {
    ctx.state.count = (ctx.state.count as number) + 1;
    Neo.log.debug("${pluginName} tick #" + ctx.state.count);
  },
});
`;

const exampleNodeTs = `// Example Node - doubles a number
export default defineNode({
  name: "Double",
  category: "${pluginName}",
  description: "Doubles the input value",
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
`;

const gitignore = `node_modules/
dist/
`;

// Create project
try {
  await mkdir(projectDir);
  await mkdir(join(projectDir, "src"));
  await mkdir(join(projectDir, "src", "services"));
  await mkdir(join(projectDir, "src", "nodes"));

  await writeFile(join(projectDir, "package.json"), packageJson);
  await writeFile(join(projectDir, "vite.config.ts"), viteConfig);
  await writeFile(join(projectDir, "tsconfig.json"), tsconfig);
  await writeFile(join(projectDir, "neo.d.ts"), neoDts);
  await writeFile(join(projectDir, "src", "services", "main.ts"), mainServiceTs);
  await writeFile(join(projectDir, "src", "nodes", "example.ts"), exampleNodeTs);
  await writeFile(join(projectDir, ".gitignore"), gitignore);

  console.log("Created files:");
  console.log("  package.json");
  console.log("  vite.config.ts");
  console.log("  tsconfig.json");
  console.log("  neo.d.ts                  - Type definitions");
  console.log("  src/services/main.ts      - Main service");
  console.log("  src/nodes/example.ts      - Example node");
  console.log("  .gitignore");
  console.log("");
  console.log("Plugin structure:");
  console.log("  src/services/*.ts  - Each file exports a service via defineService()");
  console.log("  src/nodes/*.ts     - Each file exports a node via defineNode()");
  console.log("");
  console.log("Next steps:");
  console.log(`  cd ${projectName}`);
  console.log("  pnpm install");
  console.log("  pnpm build    # Build the plugin");
  console.log("  pnpm dev      # Watch mode for development");
} catch (err) {
  if (err.code === "EEXIST") {
    console.error(`Error: Directory '${projectName}' already exists`);
  } else {
    console.error("Error:", err.message);
  }
  process.exit(1);
}
