// Neo Runtime Bootstrap
// Sets up the global Neo object and defineService/defineNode for declarative plugins.
// Each runtime loads exactly one service or node via export default defineService/defineNode.

const core = Deno.core;

// The single loaded definition (service or node) for this runtime
let _loadedDefinition = null;
let _definitionType = null; // "service" or "node"
let _serviceState = {}; // Service state persisted across ticks

// Last definition created - used to capture the definition even when export default is stripped
let _lastCreatedDefinition = null;

/**
 * Define a service - called by plugin code via export default defineService({...})
 * This validates and returns the config, also saving it to _lastCreatedDefinition
 * so it can be captured even if Vite strips the export default.
 */
globalThis.defineService = (config) => {
  if (!config.name) {
    throw new Error("Service must have a name");
  }
  // Mark as service definition
  config.__type = "service";
  // Save for capture in case export default was stripped by bundler
  _lastCreatedDefinition = config;
  return config;
};

/**
 * Define a node - called by plugin code via export default defineNode({...})
 * This validates and returns the config, also saving it to _lastCreatedDefinition
 * so it can be captured even if Vite strips the export default.
 */
globalThis.defineNode = (config) => {
  if (!config.name) {
    throw new Error("Node must have a name");
  }
  if (!config.execute) {
    throw new Error("Node must have an execute function");
  }
  // Mark as node definition
  config.__type = "node";
  // Save for capture in case export default was stripped by bundler
  _lastCreatedDefinition = config;
  return config;
};

/**
 * Get the last created definition (used by Rust wrapper when export default was stripped)
 */
globalThis.__getLastDefinition = () => _lastCreatedDefinition;

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
      throw new Error("Neo.points.subscribe not yet implemented");
    },
  },

  /**
   * Event operations - pub/sub event system.
   */
  events: {
    emit: (type, data) => {
      core.ops.op_event_emit(type, "plugin", data);
    },
    subscribe: (pattern, callback) => {
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
   * Utilities
   */
  utils: {
    now: () => core.ops.op_now(),
  },
};

/**
 * Internal API for Rust to call into JS.
 * Simplified for single-definition-per-runtime model.
 */
globalThis.__neo_internal = {
  /**
   * Set the loaded definition after evaluating the chunk.
   * Called by Rust after executing the service/node code.
   * @param {object} def - The default export from the chunk
   * @param {string} id - The service/node ID (e.g., "example/ticker")
   */
  setLoadedDefinition: (def, id) => {
    if (!def) {
      throw new Error("No default export found in chunk");
    }

    _loadedDefinition = { ...def, id };
    _definitionType = def.__type || "service"; // Default to service for backwards compat
    _serviceState = {}; // Reset state

    Neo.log.debug(`Loaded ${_definitionType}: ${id}`);
  },

  /**
   * Get the loaded definition.
   */
  getLoadedDefinition: () => _loadedDefinition,

  /**
   * Get definition metadata (for manifest generation).
   */
  getDefinitionMeta: () => {
    if (!_loadedDefinition) return null;

    if (_definitionType === "service") {
      return {
        type: "service",
        id: _loadedDefinition.id,
        name: _loadedDefinition.name,
        tickInterval: _loadedDefinition.tickInterval || null,
        subscriptions: _loadedDefinition.subscriptions || [],
      };
    } else {
      return {
        type: "node",
        id: _loadedDefinition.id,
        name: _loadedDefinition.name,
        category: _loadedDefinition.category || "Plugin",
        description: _loadedDefinition.description,
        inputs: _loadedDefinition.inputs || [],
        outputs: _loadedDefinition.outputs || [],
        pure: _loadedDefinition.pure ?? true,
        latent: _loadedDefinition.latent ?? false,
      };
    }
  },

  // --- Service Lifecycle ---

  /**
   * Start the loaded service.
   */
  startService: async () => {
    if (!_loadedDefinition || _definitionType !== "service") {
      throw new Error("No service loaded in this runtime");
    }

    if (_loadedDefinition.onStart) {
      const ctx = {
        state: _serviceState,
        config: _loadedDefinition.config || {},
      };
      await _loadedDefinition.onStart(ctx);
    }
  },

  /**
   * Stop the loaded service.
   */
  stopService: async () => {
    if (!_loadedDefinition || _definitionType !== "service") {
      throw new Error("No service loaded in this runtime");
    }

    if (_loadedDefinition.onStop) {
      const ctx = {
        state: _serviceState,
        config: _loadedDefinition.config || {},
      };
      await _loadedDefinition.onStop(ctx);
    }
  },

  /**
   * Tick the loaded service.
   */
  tickService: async () => {
    if (!_loadedDefinition || _definitionType !== "service") {
      throw new Error("No service loaded in this runtime");
    }

    if (_loadedDefinition.onTick) {
      const ctx = {
        state: _serviceState,
        config: _loadedDefinition.config || {},
      };
      await _loadedDefinition.onTick(ctx);
    }
  },

  /**
   * Send an event to the loaded service.
   */
  eventService: async (event) => {
    if (!_loadedDefinition || _definitionType !== "service") {
      throw new Error("No service loaded in this runtime");
    }

    if (_loadedDefinition.onEvent) {
      const ctx = {
        state: _serviceState,
        config: _loadedDefinition.config || {},
      };
      await _loadedDefinition.onEvent(ctx, event);
    }
  },

  // --- Node Execution ---

  /**
   * Execute the loaded node.
   * @param {object} context - The node context (inputs, config, variables)
   * @returns {Promise<object>} The node output
   */
  executeNode: async (context) => {
    if (!_loadedDefinition || _definitionType !== "node") {
      return {
        error: "No node loaded in this runtime",
        values: {},
        result: { type: "error", message: "No node loaded in this runtime" },
      };
    }

    try {
      const ctx = {
        nodeId: _loadedDefinition.id,
        config: context.config || {},
        inputs: context.inputs || {},
        variables: context.variables || {},
        getInput: (name) => context.inputs?.[name],
        getConfig: (key) => context.config?.[key],
        getVariable: (name) => context.variables?.[name],
      };

      const result = await _loadedDefinition.execute(ctx);

      if (result === undefined || result === null) {
        return { values: {}, result: { type: "end" } };
      }

      if (result.result) {
        return result;
      }

      return { values: result, result: { type: "end" } };
    } catch (err) {
      Neo.log.error(`Error executing node ${_loadedDefinition.id}: ${err.message}`);
      return {
        values: {},
        result: { type: "error", message: err.message },
      };
    }
  },
};
