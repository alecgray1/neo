// Neo Runtime Bootstrap
// Sets up the global Neo object and defineService/defineNode for declarative plugins.
//
// Supports two modes:
// 1. Single-definition mode (services): One service per runtime via setLoadedDefinition
// 2. Multi-node mode (blueprints): Multiple nodes per runtime via registerNode

const core = Deno.core;

// =============================================================================
// TIMERS (setTimeout, setInterval, clearTimeout, clearInterval)
// =============================================================================
// Uses deno_core's built-in timer system via core.queueUserTimer/cancelTimer

/**
 * setTimeout - execute callback after delay
 */
globalThis.setTimeout = function(callback, timeout = 0, ...args) {
  if (typeof callback !== "function") {
    throw new TypeError("Callback must be a function");
  }
  const wrappedCallback = () => {
    try {
      callback(...args);
    } catch (e) {
      Neo.log.error(`setTimeout callback error: ${e.message}`);
    }
  };
  return core.queueUserTimer(
    core.getTimerDepth() + 1,
    false, // not repeating
    Math.max(0, timeout),
    wrappedCallback
  );
};

/**
 * setInterval - execute callback repeatedly at interval
 */
globalThis.setInterval = function(callback, timeout = 0, ...args) {
  if (typeof callback !== "function") {
    throw new TypeError("Callback must be a function");
  }
  const wrappedCallback = () => {
    try {
      callback(...args);
    } catch (e) {
      Neo.log.error(`setInterval callback error: ${e.message}`);
    }
  };
  return core.queueUserTimer(
    core.getTimerDepth() + 1,
    true, // repeating
    Math.max(0, timeout),
    wrappedCallback
  );
};

/**
 * clearTimeout - cancel a timeout
 */
globalThis.clearTimeout = function(id) {
  if (id !== undefined && id !== null) {
    core.cancelTimer(id);
  }
};

/**
 * clearInterval - cancel an interval
 */
globalThis.clearInterval = function(id) {
  if (id !== undefined && id !== null) {
    core.cancelTimer(id);
  }
};

// --- Single-definition mode (for services, backward compat) ---
let _loadedDefinition = null;
let _definitionType = null; // "service" or "node"
let _serviceState = {}; // Service state persisted across callbacks

// --- Multi-node mode (for blueprint runtimes) ---
const _nodeRegistry = new Map(); // nodeId -> node definition

// --- Built-in node registry (separate from plugin nodes) ---
const _builtinNodes = new Map();

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
  },

  /**
   * Event operations - pub/sub event system.
   */
  events: {
    emit: (type, data) => {
      core.ops.op_event_emit(type, "plugin", data);
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

  /**
   * ECS operations - entity-component system for data modeling.
   *
   * Provides a Flecs-based ECS for managing entities with components
   * representing BACnet devices, points, and their relationships.
   */
  ecs: {
    /**
     * Create a new entity.
     * @param {Object} options - Entity creation options
     * @param {string} [options.name] - Optional entity name
     * @param {bigint|number} [options.parent] - Optional parent entity ID
     * @param {Object} [options.components] - Component data keyed by component name
     * @param {string[]} [options.tags] - Tags to add to the entity
     * @returns {Promise<bigint>} The created entity ID
     *
     * @example
     * const vav = await Neo.ecs.createEntity({
     *   name: "VAV-3-01",
     *   parent: floor3Id,
     *   components: {
     *     BacnetDevice: { device_id: 101, address: "10.0.1.50:47808" },
     *     Temperature: { value: 72.4, unit: "F" },
     *   },
     *   tags: ["VavBox", "Device"],
     * });
     */
    createEntity: async (options = {}) => {
      const { name, parent, components = {}, tags = [] } = options;
      const componentList = Object.entries(components);
      return await core.ops.op_ecs_create_entity(
        name ?? null,
        parent != null ? BigInt(parent) : null,
        componentList,
        tags
      );
    },

    /**
     * Delete an entity.
     * @param {bigint|number} entityId - Entity ID to delete
     */
    deleteEntity: async (entityId) => {
      await core.ops.op_ecs_delete_entity(BigInt(entityId));
    },

    /**
     * Get a component from an entity.
     * @param {bigint|number} entityId - Entity ID
     * @param {string} component - Component name
     * @returns {Promise<Object|null>} The component data or null
     */
    getComponent: async (entityId, component) => {
      return await core.ops.op_ecs_get_component(BigInt(entityId), component);
    },

    /**
     * Set a component on an entity.
     * @param {bigint|number} entityId - Entity ID
     * @param {string} component - Component name
     * @param {Object} data - Component data
     */
    setComponent: async (entityId, component, data) => {
      await core.ops.op_ecs_set_component(BigInt(entityId), component, data);
    },

    /**
     * Add a tag to an entity.
     * @param {bigint|number} entityId - Entity ID
     * @param {string} tag - Tag name
     */
    addTag: async (entityId, tag) => {
      await core.ops.op_ecs_add_tag(BigInt(entityId), tag);
    },

    /**
     * Remove a tag from an entity.
     * @param {bigint|number} entityId - Entity ID
     * @param {string} tag - Tag name
     */
    removeTag: async (entityId, tag) => {
      await core.ops.op_ecs_remove_tag(BigInt(entityId), tag);
    },

    /**
     * Check if an entity has a tag.
     * @param {bigint|number} entityId - Entity ID
     * @param {string} tag - Tag name
     * @returns {Promise<boolean>}
     */
    hasTag: async (entityId, tag) => {
      return await core.ops.op_ecs_has_tag(BigInt(entityId), tag);
    },

    /**
     * Look up an entity by name.
     * @param {string} name - Entity name
     * @returns {Promise<bigint|null>} Entity ID or null if not found
     */
    lookup: async (name) => {
      return await core.ops.op_ecs_lookup(name);
    },

    /**
     * Get children of an entity.
     * @param {bigint|number} entityId - Entity ID
     * @returns {Promise<bigint[]>} Array of child entity IDs
     */
    getChildren: async (entityId) => {
      return await core.ops.op_ecs_get_children(BigInt(entityId));
    },

    /**
     * Get parent of an entity.
     * @param {bigint|number} entityId - Entity ID
     * @returns {Promise<bigint|null>} Parent entity ID or null
     */
    getParent: async (entityId) => {
      return await core.ops.op_ecs_get_parent(BigInt(entityId));
    },

    /**
     * Query entities with specific components.
     * @param {Object} options - Query options
     * @param {string[]} options.with - Components entities must have
     * @returns {Promise<Object[]>} Query results
     *
     * @example
     * const vavsOnFloor3 = await Neo.ecs.query({
     *   with: ["Temperature", "VavBox"],
     * });
     */
    query: async (options = {}) => {
      const { with: components = [] } = options;
      return await core.ops.op_ecs_query(components);
    },
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

  // --- Multi-Node Registry (for blueprint runtimes) ---

  /**
   * Register a node into the registry (for multi-node mode).
   * Called by Rust after loading node code into a blueprint runtime.
   * @param {string} nodeId - The node type ID (e.g., "example/add")
   * @param {object} def - The node definition from defineNode()
   */
  registerNode: (nodeId, def) => {
    _nodeRegistry.set(nodeId, { ...def, id: nodeId });
    Neo.log.debug(`Registered node in blueprint runtime: ${nodeId}`);
  },

  /**
   * Check if a node is registered in the registry.
   * @param {string} nodeId - The node type ID
   * @returns {boolean}
   */
  hasNode: (nodeId) => _nodeRegistry.has(nodeId),

  /**
   * Get all registered node IDs.
   * @returns {string[]}
   */
  getRegisteredNodes: () => Array.from(_nodeRegistry.keys()),

  /**
   * Execute a specific node by ID from the registry.
   * Used by blueprint runtimes that have multiple nodes loaded.
   * @param {string} nodeId - The node type ID
   * @param {object} context - The node context (inputs, config, variables)
   * @returns {Promise<object>} The node output
   */
  executeNodeById: async (nodeId, context) => {
    const def = _nodeRegistry.get(nodeId);
    if (!def) {
      return {
        error: `Node not found in registry: ${nodeId}`,
        values: {},
        result: { type: "error", message: `Node not found: ${nodeId}` },
      };
    }

    try {
      const ctx = {
        nodeId,
        config: context.config || {},
        inputs: context.inputs || {},
        variables: context.variables || {},
        getInput: (name) => context.inputs?.[name],
        getConfig: (key) => context.config?.[key],
        getVariable: (name) => context.variables?.[name],
      };

      const result = await def.execute(ctx);

      if (result === undefined || result === null) {
        return { values: {}, result: { type: "end" } };
      }

      if (result.result) {
        return result;
      }

      return { values: result, result: { type: "end" } };
    } catch (err) {
      Neo.log.error(`Error executing node ${nodeId}: ${err.message}`);
      return {
        values: {},
        result: { type: "error", message: err.message },
      };
    }
  },

  // --- Single-Node Execution (for backward compat) ---

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

// =============================================================================
// BUILT-IN NODES
// =============================================================================

/**
 * Register a built-in node type.
 * Built-in nodes are available in all blueprint runtimes.
 */
function registerBuiltinNode(nodeType, def) {
  _builtinNodes.set(nodeType, { ...def, id: nodeType });
}

// --- Math Nodes ---

registerBuiltinNode("math/Add", {
  pure: true,
  execute: (ctx) => ({
    sum: (ctx.getInput("a") ?? 0) + (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("math/Subtract", {
  pure: true,
  execute: (ctx) => ({
    difference: (ctx.getInput("a") ?? 0) - (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("math/Multiply", {
  pure: true,
  execute: (ctx) => ({
    product: (ctx.getInput("a") ?? 0) * (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("math/Divide", {
  pure: true,
  execute: (ctx) => {
    const a = ctx.getInput("a") ?? 0;
    const b = ctx.getInput("b") ?? 1;
    return { quotient: b !== 0 ? a / b : 0 };
  },
});

registerBuiltinNode("math/Modulo", {
  pure: true,
  execute: (ctx) => {
    const a = ctx.getInput("a") ?? 0;
    const b = ctx.getInput("b") ?? 1;
    return { remainder: b !== 0 ? a % b : 0 };
  },
});

registerBuiltinNode("math/Negate", {
  pure: true,
  execute: (ctx) => ({
    result: -(ctx.getInput("value") ?? 0),
  }),
});

registerBuiltinNode("math/Abs", {
  pure: true,
  execute: (ctx) => ({
    result: Math.abs(ctx.getInput("value") ?? 0),
  }),
});

registerBuiltinNode("math/Min", {
  pure: true,
  execute: (ctx) => ({
    result: Math.min(ctx.getInput("a") ?? 0, ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("math/Max", {
  pure: true,
  execute: (ctx) => ({
    result: Math.max(ctx.getInput("a") ?? 0, ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("math/Clamp", {
  pure: true,
  execute: (ctx) => {
    const value = ctx.getInput("value") ?? 0;
    const min = ctx.getInput("min") ?? 0;
    const max = ctx.getInput("max") ?? 1;
    return { result: Math.max(min, Math.min(max, value)) };
  },
});

registerBuiltinNode("math/Floor", {
  pure: true,
  execute: (ctx) => ({
    result: Math.floor(ctx.getInput("value") ?? 0),
  }),
});

registerBuiltinNode("math/Ceil", {
  pure: true,
  execute: (ctx) => ({
    result: Math.ceil(ctx.getInput("value") ?? 0),
  }),
});

registerBuiltinNode("math/Round", {
  pure: true,
  execute: (ctx) => ({
    result: Math.round(ctx.getInput("value") ?? 0),
  }),
});

// --- Logic Nodes ---

registerBuiltinNode("logic/And", {
  pure: true,
  execute: (ctx) => ({
    result: Boolean(ctx.getInput("a")) && Boolean(ctx.getInput("b")),
  }),
});

registerBuiltinNode("logic/Or", {
  pure: true,
  execute: (ctx) => ({
    result: Boolean(ctx.getInput("a")) || Boolean(ctx.getInput("b")),
  }),
});

registerBuiltinNode("logic/Not", {
  pure: true,
  execute: (ctx) => ({
    result: !Boolean(ctx.getInput("value")),
  }),
});

registerBuiltinNode("logic/Xor", {
  pure: true,
  execute: (ctx) => ({
    result: Boolean(ctx.getInput("a")) !== Boolean(ctx.getInput("b")),
  }),
});

// --- Comparison Nodes ---

registerBuiltinNode("comparison/Equal", {
  pure: true,
  execute: (ctx) => ({
    result: ctx.getInput("a") === ctx.getInput("b"),
  }),
});

registerBuiltinNode("comparison/NotEqual", {
  pure: true,
  execute: (ctx) => ({
    result: ctx.getInput("a") !== ctx.getInput("b"),
  }),
});

registerBuiltinNode("comparison/Greater", {
  pure: true,
  execute: (ctx) => ({
    result: (ctx.getInput("a") ?? 0) > (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("comparison/GreaterOrEqual", {
  pure: true,
  execute: (ctx) => ({
    result: (ctx.getInput("a") ?? 0) >= (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("comparison/Less", {
  pure: true,
  execute: (ctx) => ({
    result: (ctx.getInput("a") ?? 0) < (ctx.getInput("b") ?? 0),
  }),
});

registerBuiltinNode("comparison/LessOrEqual", {
  pure: true,
  execute: (ctx) => ({
    result: (ctx.getInput("a") ?? 0) <= (ctx.getInput("b") ?? 0),
  }),
});

// --- Flow Control Nodes ---

registerBuiltinNode("flow/Branch", {
  pure: false,
  execute: (ctx) => ({
    values: {},
    result: {
      type: "continue",
      pin: ctx.getInput("condition") ? "true" : "false",
    },
  }),
});

registerBuiltinNode("flow/Sequence", {
  pure: false,
  execute: async (ctx) => {
    // Return continue to "then_0", the executor will follow up
    // Sequence nodes are special - they execute multiple branches
    return {
      values: {},
      result: { type: "sequence", pins: ["then_0", "then_1", "then_2", "then_3"] },
    };
  },
});

registerBuiltinNode("flow/ForLoop", {
  pure: false,
  execute: (ctx) => {
    // ForLoop is handled specially by the executor
    // It needs to execute the body multiple times
    const start = ctx.getInput("start") ?? 0;
    const end = ctx.getInput("end") ?? 0;
    return {
      values: { index: start },
      result: { type: "loop", pin: "body", start, end, current: start },
    };
  },
});

// --- Utility Nodes ---

registerBuiltinNode("utility/Print", {
  pure: false,
  execute: (ctx) => {
    const msg = ctx.getInput("message");
    Neo.log.info(`[Blueprint] ${msg}`);
    return { values: {}, result: { type: "continue", pin: "then" } };
  },
});

registerBuiltinNode("utility/Constant", {
  pure: true,
  execute: (ctx) => ({
    value: ctx.getConfig("value"),
  }),
});

registerBuiltinNode("utility/GetVariable", {
  pure: true,
  execute: (ctx) => ({
    value: ctx.variables[ctx.getConfig("variable")],
  }),
});

registerBuiltinNode("utility/SetVariable", {
  pure: false,
  execute: (ctx) => {
    const name = ctx.getConfig("variable");
    const value = ctx.getInput("value");
    core.ops.op_set_variable(name, value);
    return { values: { value }, result: { type: "continue", pin: "then" } };
  },
});

registerBuiltinNode("utility/Log", {
  pure: false,
  execute: (ctx) => {
    const level = ctx.getConfig("level") ?? "info";
    const msg = ctx.getInput("message");
    Neo.log[level]?.(msg) ?? Neo.log.info(msg);
    return { values: {}, result: { type: "continue", pin: "then" } };
  },
});

// --- Point Nodes ---

registerBuiltinNode("points/ReadPoint", {
  pure: false,
  execute: async (ctx) => {
    const pointId = ctx.getInput("pointId") ?? ctx.getConfig("pointId");
    const value = await Neo.points.read(pointId);
    return { value };
  },
});

registerBuiltinNode("points/WritePoint", {
  pure: false,
  execute: async (ctx) => {
    const pointId = ctx.getInput("pointId") ?? ctx.getConfig("pointId");
    const value = ctx.getInput("value");
    await Neo.points.write(pointId, value);
    return { values: {}, result: { type: "continue", pin: "then" } };
  },
});

// --- Event Nodes ---

registerBuiltinNode("event/OnStart", {
  pure: false,
  isEntry: true,
  execute: (ctx) => ({
    values: {},
    result: { type: "continue", pin: "then" },
  }),
});

registerBuiltinNode("event/OnEvent", {
  pure: false,
  isEntry: true,
  execute: (ctx) => ({
    values: { event: ctx.triggerData },
    result: { type: "continue", pin: "then" },
  }),
});

// =============================================================================
// BLUEPRINT EXECUTION ENGINE
// =============================================================================

/**
 * Look up a node definition (built-in or plugin).
 */
function getNodeDefinition(nodeType) {
  return _builtinNodes.get(nodeType) || _nodeRegistry.get(nodeType);
}

/**
 * Main entry point - called by Rust via execute_blueprint().
 * @param {object} trigger - The trigger that started execution { type, data }
 * @returns {Promise<object>} Execution result
 */
globalThis.__neo_internal.executeBlueprint = async (trigger) => {
  // First try the global (set by Rust via set_blueprint_inner)
  // Then fall back to OpState (via op_get_blueprint)
  const blueprint = globalThis.__neo_current_blueprint || core.ops.op_get_blueprint();
  if (!blueprint) {
    throw new Error("No blueprint set for execution");
  }

  Neo.log.debug(`Executing blueprint: ${blueprint.name}`);

  // Initialize execution state
  const nodeOutputs = new Map();
  const variables = { ...blueprint.variables };

  // Initialize variables in OpState
  for (const [name, value] of Object.entries(variables)) {
    core.ops.op_set_variable(name, value);
  }

  // Find entry nodes based on trigger type
  let entryNodes = [];
  const triggerType = trigger?.type ?? "start";

  for (const node of blueprint.nodes) {
    const def = getNodeDefinition(node.type);
    if (!def?.isEntry) continue;

    // Match trigger type to entry node type
    if (triggerType === "start" && node.type === "event/OnStart") {
      entryNodes.push(node);
    } else if (triggerType === "event" && node.type === "event/OnEvent") {
      entryNodes.push(node);
    }
  }

  // If no matching entry nodes, try any entry node (fallback)
  if (entryNodes.length === 0) {
    entryNodes = blueprint.nodes.filter((n) => {
      const def = getNodeDefinition(n.type);
      return def?.isEntry;
    });
  }

  // Execute from each entry node
  for (const entryNode of entryNodes) {
    await executeFromNode(
      blueprint,
      entryNode.id,
      nodeOutputs,
      variables,
      trigger?.data
    );
  }

  // Get final variables from OpState
  const finalVariables = core.ops.op_get_all_variables();

  return {
    status: "completed",
    outputs: Object.fromEntries(nodeOutputs),
    variables: finalVariables,
  };
};

/**
 * Execute from a specific node, following the execution flow.
 */
async function executeFromNode(
  blueprint,
  startNodeId,
  nodeOutputs,
  variables,
  triggerData
) {
  let currentNodeId = startNodeId;

  while (currentNodeId) {
    const node = blueprint.nodes.find((n) => n.id === currentNodeId);
    if (!node) break;

    const def = getNodeDefinition(node.type);
    if (!def) {
      Neo.log.error(`Unknown node type: ${node.type}`);
      break;
    }

    // Gather inputs from connected nodes
    const inputs = await gatherInputs(
      blueprint,
      node.id,
      nodeOutputs,
      variables
    );

    // Build execution context
    const ctx = {
      nodeId: node.id,
      nodeType: node.type,
      config: node.config || {},
      inputs,
      variables,
      triggerData,
      getInput: (name) => inputs[name],
      getConfig: (key) => node.config?.[key],
    };

    // Execute the node
    let output;
    try {
      output = await def.execute(ctx);
    } catch (err) {
      Neo.log.error(`Node ${node.id} (${node.type}) error: ${err.message}`);
      output = { values: {}, result: { type: "error", message: err.message } };
    }

    // Normalize output format
    if (output === undefined || output === null) {
      output = { values: {}, result: { type: "end" } };
    } else if (!output.result && !output.values) {
      // Pure node returned just values
      output = { values: output, result: { type: "end" } };
    } else if (!output.result) {
      output = { ...output, result: { type: "end" } };
    }

    // Store outputs
    nodeOutputs.set(node.id, output.values || {});

    // Sync variables with OpState
    variables = core.ops.op_get_all_variables();

    // Determine next node based on result
    const result = output.result;

    switch (result.type) {
      case "continue":
        currentNodeId = findConnectedExecNode(blueprint, node.id, result.pin);
        break;

      case "sequence":
        // Execute each branch in sequence
        for (const pin of result.pins || []) {
          const nextId = findConnectedExecNode(blueprint, node.id, pin);
          if (nextId) {
            await executeFromNode(
              blueprint,
              nextId,
              nodeOutputs,
              variables,
              triggerData
            );
          }
        }
        currentNodeId = null;
        break;

      case "loop":
        // Execute loop body repeatedly
        for (let i = result.start; i < result.end; i++) {
          // Update loop index in outputs
          nodeOutputs.set(node.id, { index: i });
          core.ops.op_set_variable("__loop_index", i);

          const bodyId = findConnectedExecNode(blueprint, node.id, result.pin);
          if (bodyId) {
            await executeFromNode(
              blueprint,
              bodyId,
              nodeOutputs,
              variables,
              triggerData
            );
          }
        }
        // After loop, continue to "completed" pin
        currentNodeId = findConnectedExecNode(blueprint, node.id, "completed");
        break;

      case "error":
        Neo.log.error(`Execution error at ${node.id}: ${result.message}`);
        currentNodeId = null;
        break;

      case "end":
      default:
        currentNodeId = null;
        break;
    }
  }
}

/**
 * Gather input values for a node from its connections.
 */
async function gatherInputs(blueprint, nodeId, nodeOutputs, variables) {
  const inputs = {};

  for (const conn of blueprint.connections) {
    // Parse "nodeId.pinName" format
    const [toNode, toPin] = parseConnectionPin(conn.to);
    if (toNode !== nodeId) continue;

    const [fromNode, fromPin] = parseConnectionPin(conn.from);

    // Check if source already computed
    if (nodeOutputs.has(fromNode)) {
      inputs[toPin] = nodeOutputs.get(fromNode)[fromPin];
    } else {
      // Recursively evaluate pure/data nodes on demand
      inputs[toPin] = await evaluateDataNode(
        blueprint,
        fromNode,
        fromPin,
        nodeOutputs,
        variables
      );
    }
  }

  return inputs;
}

/**
 * Evaluate a pure/data node on demand (lazy evaluation).
 */
async function evaluateDataNode(
  blueprint,
  nodeId,
  outputPin,
  nodeOutputs,
  variables
) {
  // Check cache first
  if (nodeOutputs.has(nodeId)) {
    return nodeOutputs.get(nodeId)[outputPin];
  }

  const node = blueprint.nodes.find((n) => n.id === nodeId);
  if (!node) return undefined;

  const def = getNodeDefinition(node.type);
  if (!def) return undefined;

  // Only evaluate pure nodes on demand; non-pure nodes must be in the exec flow
  if (!def.pure) return undefined;

  // Gather inputs for this node
  const inputs = await gatherInputs(blueprint, nodeId, nodeOutputs, variables);

  // Build context
  const ctx = {
    nodeId: node.id,
    nodeType: node.type,
    config: node.config || {},
    inputs,
    variables,
    getInput: (name) => inputs[name],
    getConfig: (key) => node.config?.[key],
  };

  // Execute
  let output;
  try {
    output = await def.execute(ctx);
  } catch (err) {
    Neo.log.error(`Pure node ${node.id} error: ${err.message}`);
    return undefined;
  }

  // Normalize and cache
  const values = output?.values ?? output ?? {};
  nodeOutputs.set(nodeId, values);

  return values[outputPin];
}

/**
 * Find the node connected to a specific exec output pin.
 */
function findConnectedExecNode(blueprint, nodeId, pin) {
  const fromKey = `${nodeId}.${pin}`;

  for (const conn of blueprint.connections) {
    if (conn.from === fromKey) {
      const [toNode] = parseConnectionPin(conn.to);
      return toNode;
    }
  }

  return null;
}

/**
 * Parse a connection pin string "nodeId.pinName" into parts.
 */
function parseConnectionPin(connStr) {
  const dotIdx = connStr.lastIndexOf(".");
  if (dotIdx === -1) return [connStr, ""];
  return [connStr.substring(0, dotIdx), connStr.substring(dotIdx + 1)];
}
