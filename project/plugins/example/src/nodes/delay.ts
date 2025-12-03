// Delay Node - waits for a specified duration
export default defineNode({
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
