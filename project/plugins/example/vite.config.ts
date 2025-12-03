import { defineConfig } from "vite";
import neo from "@neo/vite-plugin";

export default defineConfig({
  plugins: [
    neo({
      id: "example",
      name: "Example Plugin",
      description: "Example plugin demonstrating services and nodes",
    }),
  ],
});
