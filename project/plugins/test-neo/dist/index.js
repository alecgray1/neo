globalThis.onStart = async (o) => {
  Neo.log.info("Test Neo started!"), Neo.log.debug(`Config: ${JSON.stringify(Neo.config)}`);
};
globalThis.onStop = async () => {
  Neo.log.info("Test Neo stopping...");
};
globalThis.onEvent = async (o) => {
  Neo.log.debug(`Event: ${o.type}`);
};
globalThis.onTick = async () => {
};
