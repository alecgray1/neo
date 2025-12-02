import { defineConfig } from "vite";
import neo from "@neo/vite-plugin";

export default defineConfig({
  plugins: [
    neo({
      id: "my-plugin",
      name: "My Plugin",
      description: "A Neo plugin",
      // Subscribe to events (optional)
      // subscriptions: ["temperature/*"],
      // Tick interval in ms (optional)
      // tickInterval: 1000,
      // Plugin config passed to onStart (optional)
      // config: { someOption: true },
    }),
  ],
});
