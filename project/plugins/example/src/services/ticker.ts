// Ticker Service - demonstrates periodic tick callbacks
export default defineService({
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
