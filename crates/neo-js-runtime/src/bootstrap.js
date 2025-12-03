// Neo Runtime Bootstrap
// Sets up the global Neo object that plugins use to interact with the host.

const core = Deno.core;

// Node registry for blueprint nodes
const nodeRegistry = new Map();

// Service registry for background services
const serviceRegistry = new Map();

// Type registry for custom types
const typeRegistry = new Map();

/**
 * The global Neo object - primary API for plugins.
 */
globalThis.Neo = {
  /**
   * Point operations - read/write data points.
   */
  points: {
    read: async (id) => {
      return await core.ops.op_point_read(id);
    },
    write: async (id, value) => {
      await core.ops.op_point_write(id, value);
    },
    subscribe: (pattern, callback) => {
      // Subscriptions require event loop integration - not yet implemented
      throw new Error("Neo.points.subscribe not yet implemented");
    },
  },

  /**
   * Event operations - pub/sub event system.
   */
  events: {
    emit: (type, data) => {
      // Source is set to the plugin ID by the runtime
      core.ops.op_event_emit(type, "plugin", data);
    },
    subscribe: (pattern, callback) => {
      // Subscriptions require event loop integration - not yet implemented
      throw new Error("Neo.events.subscribe not yet implemented");
    },
  },

  /**
   * Logging - writes to the Rust tracing system.
   */
  log: {
    error: (msg) => core.ops.op_log("error", String(msg)),
    warn: (msg) => core.ops.op_log("warn", String(msg)),
    info: (msg) => core.ops.op_log("info", String(msg)),
    debug: (msg) => core.ops.op_log("debug", String(msg)),
    trace: (msg) => core.ops.op_log("trace", String(msg)),
  },

  /**
   * Node registration - plugins register blueprint nodes here.
   */
  nodes: {
    register: (def) => {
      if (!def.id) {
        throw new Error("Node definition must have an id");
      }
      if (!def.execute) {
        throw new Error("Node definition must have an execute function");
      }
      nodeRegistry.set(def.id, def);
      Neo.log.debug(`Registered node: ${def.id}`);
    },

    get: (id) => nodeRegistry.get(id),

    list: () => Array.from(nodeRegistry.keys()),

    getAll: () => Object.fromEntries(nodeRegistry),
  },

  /**
   * Service registration - plugins register background services here.
   */
  services: {
    register: (def) => {
      if (!def.id) {
        throw new Error("Service definition must have an id");
      }
      serviceRegistry.set(def.id, def);
      Neo.log.debug(`Registered service: ${def.id}`);
    },

    get: (id) => serviceRegistry.get(id),

    list: () => Array.from(serviceRegistry.keys()),

    getAll: () => Object.fromEntries(serviceRegistry),
  },

  /**
   * Type registration - plugins register custom types here.
   */
  types: {
    register: (def) => {
      if (!def.id) {
        throw new Error("Type definition must have an id");
      }
      typeRegistry.set(def.id, def);
      Neo.log.debug(`Registered type: ${def.id}`);
    },

    get: (id) => typeRegistry.get(id),

    list: () => Array.from(typeRegistry.keys()),

    getAll: () => Object.fromEntries(typeRegistry),
  },

  /**
   * Utilities
   */
  utils: {
    now: () => core.ops.op_now(),
  },
};

/**
 * Internal API for Rust to call into JS.
 * This is used by the blueprint executor to run plugin nodes.
 */
globalThis.__neo_internal = {
  /**
   * Start a specific service by ID.
   * @param {string} id - The service ID
   */
  startService: async (id) => {
    const service = serviceRegistry.get(id);
    if (!service) {
      throw new Error(`Service not found: ${id}`);
    }
    if (service.onStart) {
      // Create service context with state
      if (!service._state) {
        service._state = {};
      }
      const ctx = { state: service._state };
      await service.onStart(ctx);
    }
  },

  /**
   * Stop a specific service by ID.
   * @param {string} id - The service ID
   */
  stopService: async (id) => {
    const service = serviceRegistry.get(id);
    if (!service) {
      throw new Error(`Service not found: ${id}`);
    }
    if (service.onStop) {
      const ctx = { state: service._state || {} };
      await service.onStop(ctx);
    }
  },

  /**
   * Tick a specific service by ID.
   * @param {string} id - The service ID
   */
  tickService: async (id) => {
    const service = serviceRegistry.get(id);
    if (!service) {
      throw new Error(`Service not found: ${id}`);
    }
    if (service.onTick) {
      const ctx = { state: service._state || {} };
      await service.onTick(ctx);
    }
  },

  /**
   * Send an event to a specific service by ID.
   * @param {string} id - The service ID
   * @param {object} event - The event object
   */
  eventService: async (id, event) => {
    const service = serviceRegistry.get(id);
    if (!service) {
      throw new Error(`Service not found: ${id}`);
    }
    if (service.onEvent) {
      const ctx = { state: service._state || {} };
      await service.onEvent(ctx, event);
    }
  },

  /**
   * Get list of registered services with their definitions.
   * Called by Rust during scan phase to discover what services a plugin provides.
   */
  getServiceDefinitions: () => {
    const defs = [];
    for (const [id, def] of serviceRegistry) {
      defs.push({
        id: def.id,
        name: def.name || def.id,
        subscriptions: def.subscriptions || [],
        tickInterval: def.tickInterval || null,
      });
    }
    return defs;
  },

  /**
   * Execute a registered node.
   * Called by Rust via execute_node_in_js.
   *
   * @param {string} nodeId - The node ID (e.g., "myPlugin/HttpGet")
   * @param {object} context - The node context (nodeId, config, inputs, variables)
   * @returns {Promise<object>} The node output (values, result)
   */
  executeNode: async (nodeId, context) => {
    const nodeDef = nodeRegistry.get(nodeId);
    if (!nodeDef) {
      return {
        error: `Node not found: ${nodeId}`,
        values: {},
        result: { type: "error", message: `Node not found: ${nodeId}` },
      };
    }

    try {
      // Build the context object passed to the execute function
      const ctx = {
        nodeId: context.nodeId,
        config: context.config || {},
        inputs: context.inputs || {},
        variables: context.variables || {},
        // Helper methods
        getInput: (name) => context.inputs?.[name],
        getConfig: (key) => context.config?.[key],
        getVariable: (name) => context.variables?.[name],
      };

      // Call the node's execute function
      const result = await nodeDef.execute(ctx);

      // Normalize the result
      if (result === undefined || result === null) {
        return { values: {}, result: { type: "end" } };
      }

      // If result has explicit structure, use it
      if (result.result) {
        return result;
      }

      // Otherwise treat it as pure output values
      return { values: result, result: { type: "end" } };
    } catch (err) {
      Neo.log.error(`Error executing node ${nodeId}: ${err.message}`);
      return {
        values: {},
        result: { type: "error", message: err.message },
      };
    }
  },

  /**
   * Get list of registered nodes with their definitions.
   * Called by Rust to discover what nodes a plugin provides.
   */
  getNodeDefinitions: () => {
    const defs = [];
    for (const [id, def] of nodeRegistry) {
      defs.push({
        id: def.id,
        name: def.name || def.id,
        category: def.category || "Plugin",
        description: def.description,
        inputs: def.inputs || [],
        outputs: def.outputs || [],
        pure: def.pure ?? true,
        latent: def.latent ?? false,
      });
    }
    return defs;
  },
};
