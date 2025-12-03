// Add Node - adds two numbers together
export default defineNode({
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
