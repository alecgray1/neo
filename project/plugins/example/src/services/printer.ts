// Printer Service - demonstrates using setInterval for periodic work
export default defineService({
  name: "Printer",

  onStart: async (ctx) => {
    ctx.state.count = 0;

    // Use JS setInterval - the V8 event loop handles this
    ctx.state.intervalId = setInterval(() => {
      ctx.state.count += 1;
      Neo.log.info(`[Printer] Hello from JS! Count: ${ctx.state.count}`);
    }, 5000);

    Neo.log.info("[Printer] Service started - will print every 5 seconds");
  },

  onStop: async (ctx) => {
    if (ctx.state.intervalId) {
      clearInterval(ctx.state.intervalId);
    }
    Neo.log.info(`[Printer] Service stopped after ${ctx.state.count} prints`);
  },
});
