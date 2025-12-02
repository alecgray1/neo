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
    "dev": "vite build --watch",
    "build": "vite build",
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
      // Subscribe to events (optional)
      // subscriptions: ["temperature/*"],
      // Tick interval in ms (optional)
      // tickInterval: 1000,
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
    "lib": ["ES2022"]
  },
  "include": ["src/**/*", "neo.d.ts"]
}
`;

const neoDts = `/**
 * Neo Plugin API Type Definitions
 */

declare global {
  /**
   * Neo global object available to all plugins
   */
  const Neo: {
    log: {
      trace(message: string): void;
      debug(message: string): void;
      info(message: string): void;
      warn(message: string): void;
      error(message: string): void;
    };

    events: {
      publish(event: { type: string; data?: unknown }): void;
    };

    points: {
      read(pointId: string): Promise<unknown>;
      write(pointId: string, value: unknown): Promise<void>;
    };

    config: Record<string, unknown>;
  };

  function onStart(payload?: { config?: Record<string, unknown> }): void | Promise<void>;
  function onStop(): void | Promise<void>;
  function onEvent(event: { type: string; source: string; data?: unknown }): void | Promise<void>;
  function onTick(): void | Promise<void>;
}

export {};
`;

const indexTs = `/**
 * ${pluginName}
 *
 * A Neo plugin.
 */

// Called when the plugin starts
globalThis.onStart = async (payload) => {
  Neo.log.info("${pluginName} started!");
  Neo.log.debug(\`Config: \${JSON.stringify(Neo.config)}\`);
};

// Called when the plugin stops
globalThis.onStop = async () => {
  Neo.log.info("${pluginName} stopping...");
};

// Called when an event is received
globalThis.onEvent = async (event) => {
  Neo.log.debug(\`Event: \${event.type}\`);
};

// Called on tick interval (if configured)
globalThis.onTick = async () => {
  // Periodic tasks here
};
`;

const gitignore = `node_modules/
dist/
`;

// Create project
try {
  await mkdir(projectDir);
  await mkdir(join(projectDir, "src"));

  await writeFile(join(projectDir, "package.json"), packageJson);
  await writeFile(join(projectDir, "vite.config.ts"), viteConfig);
  await writeFile(join(projectDir, "tsconfig.json"), tsconfig);
  await writeFile(join(projectDir, "neo.d.ts"), neoDts);
  await writeFile(join(projectDir, "src", "index.ts"), indexTs);
  await writeFile(join(projectDir, ".gitignore"), gitignore);

  console.log("Created files:");
  console.log("  package.json");
  console.log("  vite.config.ts");
  console.log("  tsconfig.json");
  console.log("  neo.d.ts");
  console.log("  src/index.ts");
  console.log("  .gitignore");
  console.log("");
  console.log("Next steps:");
  console.log(`  cd ${projectName}`);
  console.log("  npm install");
  console.log("  npm run dev");
  console.log("");
  console.log("Make sure Neo server is running (cargo run) to connect.");
} catch (err) {
  if (err.code === "EEXIST") {
    console.error(`Error: Directory '${projectName}' already exists`);
  } else {
    console.error("Error:", err.message);
  }
  process.exit(1);
}
