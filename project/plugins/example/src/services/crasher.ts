// Crasher Service - tests isolation by crashing in various ways
export default defineService({
  name: "Crasher",
  tickInterval: 3000,

  onStart: async (ctx) => {
    ctx.state.tickCount = 0;
    Neo.log.info("Crasher service started - will crash on tick #3");
  },

  onStop: async (ctx) => {
    Neo.log.info("Crasher service stopped");
  },

  onTick: async (ctx) => {
    ctx.state.tickCount += 1;
    Neo.log.info("Crasher tick #" + ctx.state.tickCount);

    if (ctx.state.tickCount === 3) {
      Neo.log.warn("Crasher: About to throw an exception!");
      throw new Error("Intentional crash to test isolation!");
    }
  },
});
