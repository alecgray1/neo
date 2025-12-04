// Crasher Service - tests isolation by crashing in various ways
export default defineService({
  name: "Crasher",

  onStart: async (ctx) => {
    ctx.state.tickCount = 0;
    Neo.log.info("Crasher service started - will crash on tick #3");

    ctx.state.intervalId = setInterval(() => {
      ctx.state.tickCount += 1;
      Neo.log.info("Crasher tick #" + ctx.state.tickCount);

      if (ctx.state.tickCount === 3) {
        Neo.log.warn("Crasher: About to throw an exception!");
        throw new Error("Intentional crash to test isolation!");
      }
    }, 3000);
  },

  onStop: async (ctx) => {
    if (ctx.state.intervalId) {
      clearInterval(ctx.state.intervalId);
    }
    Neo.log.info("Crasher service stopped");
  },
});
