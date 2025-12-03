// Example 2 Plugin - Single service

Neo.services.register({
  id: "example2/watchdog",
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

Neo.log.info("Example2 plugin loaded - registered 1 service");
