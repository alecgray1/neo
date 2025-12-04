// Ticker Service - demonstrates periodic callbacks using setInterval
export default defineService({
  name: "Example Ticker",

  onStart: async (ctx) => {
    ctx.state.count = 0;
    Neo.log.info("Ticker service started");

    // Use setInterval for periodic work
    ctx.state.intervalId = setInterval(() => {
      ctx.state.count += 1;
      Neo.log.debug("Tick #" + ctx.state.count);
    }, 5000);
  },

  onStop: async (ctx) => {
    // Clean up the interval
    if (ctx.state.intervalId) {
      clearInterval(ctx.state.intervalId);
    }
    Neo.log.info("Ticker service stopped after " + ctx.state.count + " ticks");
  },
});
