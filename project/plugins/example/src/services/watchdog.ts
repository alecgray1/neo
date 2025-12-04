// Watchdog Service - monitors system health
export default defineService({
  name: "System Watchdog",

  onStart: async (ctx) => {
    ctx.state.startTime = Date.now();
    ctx.state.checks = 0;
    Neo.log.info("Watchdog service started");

    ctx.state.intervalId = setInterval(() => {
      ctx.state.checks += 1;
      const uptime = Math.floor((Date.now() - ctx.state.startTime) / 1000);
      Neo.log.debug("Watchdog check #" + ctx.state.checks + " - uptime: " + uptime + "s");
    }, 10000);
  },

  onStop: async (ctx) => {
    if (ctx.state.intervalId) {
      clearInterval(ctx.state.intervalId);
    }
    const uptime = Math.floor((Date.now() - ctx.state.startTime) / 1000);
    Neo.log.info("Watchdog stopped - uptime: " + uptime + "s, checks: " + ctx.state.checks);
  },
});
