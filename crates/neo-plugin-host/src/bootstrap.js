// Neo Plugin Bootstrap
// This file sets up the Neo global object available to all plugins

const core = Deno.core;

// V8 binary serialization helpers
const v8Serialize = (value) => core.serialize(value);
const v8Deserialize = (buffer) => core.deserialize(buffer);

// Timer implementation
const timers = new Map();
let nextTimerId = 1;

globalThis.setTimeout = (callback, delay = 0, ...args) => {
  const id = nextTimerId++;
  const controller = { cancelled: false };
  timers.set(id, controller);

  (async () => {
    await core.ops.op_sleep(Math.max(0, delay));
    if (!controller.cancelled) {
      timers.delete(id);
      callback(...args);
    }
  })();

  return id;
};

globalThis.clearTimeout = (id) => {
  const controller = timers.get(id);
  if (controller) {
    controller.cancelled = true;
    timers.delete(id);
  }
};

globalThis.setInterval = (callback, delay = 0, ...args) => {
  const id = nextTimerId++;
  const controller = { cancelled: false };
  timers.set(id, controller);

  (async () => {
    while (!controller.cancelled) {
      await core.ops.op_sleep(Math.max(0, delay));
      if (!controller.cancelled) {
        callback(...args);
      }
    }
  })();

  return id;
};

globalThis.clearInterval = (id) => {
  const controller = timers.get(id);
  if (controller) {
    controller.cancelled = true;
    timers.delete(id);
  }
};

// Neo global object
globalThis.Neo = {
  log: {
    trace: (msg) => core.ops.op_neo_log("trace", String(msg)),
    debug: (msg) => core.ops.op_neo_log("debug", String(msg)),
    info: (msg) => core.ops.op_neo_log("info", String(msg)),
    warn: (msg) => core.ops.op_neo_log("warn", String(msg)),
    error: (msg) => core.ops.op_neo_log("error", String(msg)),
  },

  events: {
    /**
     * Publish an event to the event bus (uses V8 binary serialization)
     */
    publish: (event) => {
      if (!event || typeof event.type !== "string") {
        throw new Error("Event must have a 'type' property");
      }
      const serialized = v8Serialize(event.data ?? null);
      core.ops.op_emit_v8(event.type, serialized);
    },
  },

  points: {
    /**
     * Read a point value by ID (uses V8 binary serialization)
     * @param {string} pointId - The point identifier
     * @returns {Promise<any>} The point value
     */
    read: async (pointId) => {
      const buffer = await core.ops.op_point_read_v8(String(pointId));
      return v8Deserialize(buffer);
    },

    /**
     * Write a value to a point by ID (uses V8 binary serialization)
     * @param {string} pointId - The point identifier
     * @param {any} value - The value to write
     * @returns {Promise<void>}
     */
    write: async (pointId, value) => {
      const serialized = v8Serialize(value);
      await core.ops.op_point_write_v8(String(pointId), serialized);
    },
  },

  // Config will be set by the host when Start message is received
  config: {},
};

// Console shim that uses Neo.log
globalThis.console = {
  log: (...args) => Neo.log.info(args.map(String).join(" ")),
  info: (...args) => Neo.log.info(args.map(String).join(" ")),
  warn: (...args) => Neo.log.warn(args.map(String).join(" ")),
  error: (...args) => Neo.log.error(args.map(String).join(" ")),
  debug: (...args) => Neo.log.debug(args.map(String).join(" ")),
};
