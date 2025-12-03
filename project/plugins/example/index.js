// Example Neo Plugin
// Demonstrates registering services and blueprint nodes

// Service 1: A ticker that logs periodically
Neo.services.register({
  id: "example/ticker",
  name: "Example Ticker",
  tickInterval: 5000,

  onStart: async (ctx) => {
    ctx.state.count = 0;
    Neo.log.info("Ticker service started");
  },

  onStop: async (ctx) => {
    Neo.log.info("Ticker service stopped after " + ctx.state.count + " ticks");
  },

  onTick: async (ctx) => {
    ctx.state.count += 1;
    Neo.log.debug("Tick #" + ctx.state.count);
  },
});

// Service 2: A watchdog that monitors system health
Neo.services.register({
  id: "example/watchdog",
  name: "System Watchdog",
  tickInterval: 10000,

  onStart: async (ctx) => {
    ctx.state.startTime = Date.now();
    ctx.state.checks = 0;
    Neo.log.info("Watchdog service started");
  },

  onStop: async (ctx) => {
    const uptime = Math.floor((Date.now() - ctx.state.startTime) / 1000);
    Neo.log.info("Watchdog stopped - uptime: " + uptime + "s, checks: " + ctx.state.checks);
  },

  onTick: async (ctx) => {
    ctx.state.checks += 1;
    const uptime = Math.floor((Date.now() - ctx.state.startTime) / 1000);
    Neo.log.debug("Watchdog check #" + ctx.state.checks + " - uptime: " + uptime + "s");
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

Neo.log.info("Example plugin loaded - registered 2 services and 2 nodes");
