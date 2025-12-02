/**
 * Neo Plugin Template
 *
 * This is a starter template for creating Neo plugins.
 * Plugins run in isolated processes and communicate with
 * the Neo runtime via the Neo global object.
 */

// Plugin lifecycle: Called when the plugin starts
globalThis.onStart = async (payload) => {
  Neo.log.info("Plugin started!");

  // Access configuration passed from Neo
  const config = Neo.config;
  Neo.log.debug(`Config: ${JSON.stringify(config)}`);

  // Example: Read a point value
  // const temperature = await Neo.points.read("room/temperature");

  // Example: Write a point value
  // await Neo.points.write("room/setpoint", 72);
};

// Plugin lifecycle: Called when the plugin stops
globalThis.onStop = async () => {
  Neo.log.info("Plugin stopping...");
  // Cleanup resources here
};

// Plugin lifecycle: Called when an event is received
globalThis.onEvent = async (event) => {
  Neo.log.debug(`Received event: ${event.type} from ${event.source}`);

  // Handle specific event types
  switch (event.type) {
    case "temperature:changed":
      // Handle temperature change
      break;
    default:
      // Unknown event type
      break;
  }
};

// Plugin lifecycle: Called on tick interval (if configured)
globalThis.onTick = async () => {
  // Periodic tasks here
  // Neo.log.trace("Tick");
};
