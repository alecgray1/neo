// Example Neo Plugin
// Demonstrates registering a service and blueprint nodes

// Register a simple service
Neo.services.register({
  id: "example/ticker",
  name: "Example Ticker Service",

  // Called when service starts
  onStart: async () => {
    Neo.log.info("Example ticker service started!");
  },

  // Called when service stops
  onStop: async () => {
    Neo.log.info("Example ticker service stopped");
  },

  // Called on each tick (if tick_interval is set)
  onTick: async () => {
    Neo.log.debug("Tick!");
  },
});

// Register a blueprint node that adds two numbers
Neo.nodes.register({
  id: "example/Add",
  name: "Add Numbers",
  category: "Math",
  description: "Adds two numbers together",
  inputs: [
    { name: "a", type: "number" },
    { name: "b", type: "number" },
  ],
  outputs: [
    { name: "sum", type: "number" },
  ],
  pure: true,

  execute: async (ctx) => {
    const a = ctx.getInput("a") || 0;
    const b = ctx.getInput("b") || 0;
    return { sum: a + b };
  },
});

// Register a node that fetches data (latent/async)
Neo.nodes.register({
  id: "example/Delay",
  name: "Delay",
  category: "Flow",
  description: "Waits for a specified duration",
  inputs: [
    { name: "ms", type: "number" },
  ],
  outputs: [],
  pure: false,
  latent: true,

  execute: async (ctx) => {
    const ms = ctx.getInput("ms") || 1000;
    await new Promise(resolve => setTimeout(resolve, ms));
    return {};
  },
});

Neo.log.info("Example plugin loaded - registered 1 service and 2 nodes");
