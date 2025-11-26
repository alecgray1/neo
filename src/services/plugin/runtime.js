// Neo Plugin Runtime Bootstrap
// This file sets up the global Neo API for plugins

((globalThis) => {
    const core = Deno.core;

    // ─────────────────────────────────────────────────────────────────────────
    // Neo SDK - Clean API for plugins
    // ─────────────────────────────────────────────────────────────────────────

    const Neo = {
        // Plugin info
        get pluginId() {
            return core.ops.op_neo_get_plugin_id();
        },

        // Configuration
        get config() {
            return core.ops.op_neo_get_config();
        },

        // Point operations
        points: {
            /**
             * Read a point value
             * @param {string} path - Point path like "network/device/AI:1" or "virtual/weather/temp"
             * @returns {Promise<PointValue>}
             */
            read: async (path) => {
                return await core.ops.op_neo_point_read(path);
            },

            /**
             * Write a point value
             * @param {string} path - Point path
             * @param {PointValue} value - Value to write (e.g., { Real: 72.5 } or { Boolean: true })
             */
            write: (path, value) => {
                core.ops.op_neo_point_write(path, value);
            },
        },

        // Event operations
        events: {
            /**
             * Publish an event to the Neo event bus
             * @param {NeoEvent} event - Event to publish
             */
            publish: (event) => {
                // Ensure event has required structure
                if (!event.type) {
                    throw new Error("Event must have a 'type' field");
                }
                core.ops.op_neo_event_publish(event);
            },
        },

        // Logging utilities
        log: {
            trace: (msg) => core.ops.op_neo_log("trace", String(msg)),
            debug: (msg) => core.ops.op_neo_log("debug", String(msg)),
            info: (msg) => core.ops.op_neo_log("info", String(msg)),
            warn: (msg) => core.ops.op_neo_log("warn", String(msg)),
            error: (msg) => core.ops.op_neo_log("error", String(msg)),
        },

        // Time utilities
        time: {
            /**
             * Get current timestamp in milliseconds
             * @returns {number}
             */
            now: () => core.ops.op_neo_now_ms(),
        },
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Plugin lifecycle management (multi-plugin support)
    // ─────────────────────────────────────────────────────────────────────────

    // Store registered plugin instances by ID
    globalThis.__neo_plugins = new Map();

    // For backwards compatibility, also store single plugin instance
    globalThis.__neo_plugin_instance = null;

    /**
     * Register a plugin with the Neo runtime
     * @param {ServicePlugin} plugin - Plugin implementation
     * @param {string} [pluginId] - Optional plugin ID (uses current context if not provided)
     */
    globalThis.__neo_register_plugin = (plugin, pluginId) => {
        const id = pluginId || Neo.pluginId;
        globalThis.__neo_plugins.set(id, plugin);
        // For backwards compatibility
        globalThis.__neo_plugin_instance = plugin;
        Neo.log.debug(`Plugin '${id}' registered`);
    };

    /**
     * Called by Rust to start a specific plugin
     * @param {string} pluginId - Plugin ID to start
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_start_plugin = async (pluginId) => {
        const plugin = globalThis.__neo_plugins.get(pluginId);
        if (plugin?.onStart) {
            try {
                await plugin.onStart();
            } catch (error) {
                Neo.log.error(`onStart error for '${pluginId}': ${error}`);
                throw error;
            }
        }
    };

    /**
     * Called by Rust to stop a specific plugin
     * @param {string} pluginId - Plugin ID to stop
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_stop_plugin = async (pluginId) => {
        const plugin = globalThis.__neo_plugins.get(pluginId);
        if (plugin?.onStop) {
            try {
                await plugin.onStop();
            } catch (error) {
                Neo.log.error(`onStop error for '${pluginId}': ${error}`);
                throw error;
            }
        }
        globalThis.__neo_plugins.delete(pluginId);
    };

    /**
     * Called by Rust when an event is received for a specific plugin
     * @param {string} pluginId - Plugin ID
     * @param {NeoEvent} event
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_event_for_plugin = async (pluginId, event) => {
        const plugin = globalThis.__neo_plugins.get(pluginId);
        if (plugin?.onEvent) {
            try {
                await plugin.onEvent(event);
            } catch (error) {
                Neo.log.error(`onEvent error for '${pluginId}': ${error}`);
            }
        }
    };

    /**
     * Called by Rust when a request is received for a specific plugin
     * @param {string} pluginId - Plugin ID
     * @param {ServiceRequest} request
     * @returns {Promise<ServiceResponse>}
     */
    globalThis.__neo_call_request_for_plugin = async (pluginId, request) => {
        const plugin = globalThis.__neo_plugins.get(pluginId);
        if (plugin?.onRequest) {
            try {
                return await plugin.onRequest(request);
            } catch (error) {
                Neo.log.error(`onRequest error for '${pluginId}': ${error}`);
                return {
                    type: "Error",
                    code: "PLUGIN_ERROR",
                    message: String(error),
                };
            }
        }
        return {
            type: "Error",
            code: "NOT_IMPLEMENTED",
            message: "onRequest not implemented",
        };
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Backwards compatible single-plugin functions
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Called by Rust to start the plugin (single-plugin mode)
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_start = async () => {
        const plugin = globalThis.__neo_plugin_instance;
        if (plugin?.onStart) {
            try {
                await plugin.onStart();
            } catch (error) {
                Neo.log.error(`onStart error: ${error}`);
                throw error;
            }
        }
    };

    /**
     * Called by Rust to stop the plugin (single-plugin mode)
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_stop = async () => {
        const plugin = globalThis.__neo_plugin_instance;
        if (plugin?.onStop) {
            try {
                await plugin.onStop();
            } catch (error) {
                Neo.log.error(`onStop error: ${error}`);
                throw error;
            }
        }
    };

    /**
     * Called by Rust when an event is received (single-plugin mode)
     * @param {NeoEvent} event
     * @returns {Promise<void>}
     */
    globalThis.__neo_call_event = async (event) => {
        const plugin = globalThis.__neo_plugin_instance;
        if (plugin?.onEvent) {
            try {
                await plugin.onEvent(event);
            } catch (error) {
                Neo.log.error(`onEvent error: ${error}`);
            }
        }
    };

    /**
     * Called by Rust when a request is received (single-plugin mode)
     * @param {ServiceRequest} request
     * @returns {Promise<ServiceResponse>}
     */
    globalThis.__neo_call_request = async (request) => {
        const plugin = globalThis.__neo_plugin_instance;
        if (plugin?.onRequest) {
            try {
                return await plugin.onRequest(request);
            } catch (error) {
                Neo.log.error(`onRequest error: ${error}`);
                return {
                    type: "Error",
                    code: "PLUGIN_ERROR",
                    message: String(error),
                };
            }
        }
        return {
            type: "Error",
            code: "NOT_IMPLEMENTED",
            message: "onRequest not implemented",
        };
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Helper function to define a service plugin
    // ─────────────────────────────────────────────────────────────────────────

    /**
     * Define and register a Neo service plugin
     * @param {ServicePlugin} plugin
     */
    globalThis.defineService = (plugin) => {
        __neo_register_plugin(plugin);
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Console override to route through Neo logging
    // ─────────────────────────────────────────────────────────────────────────

    globalThis.console = {
        log: (...args) => Neo.log.info(args.map(String).join(" ")),
        info: (...args) => Neo.log.info(args.map(String).join(" ")),
        warn: (...args) => Neo.log.warn(args.map(String).join(" ")),
        error: (...args) => Neo.log.error(args.map(String).join(" ")),
        debug: (...args) => Neo.log.debug(args.map(String).join(" ")),
        trace: (...args) => Neo.log.trace(args.map(String).join(" ")),
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Timer polyfills (stub implementation)
    // Note: Real timers are handled by the Rust runtime's periodic event loop ticks
    // ─────────────────────────────────────────────────────────────────────────

    let timerId = 0;
    const pendingTimers = new Map();
    const pendingIntervals = new Map();

    // Process pending timers - called periodically by the Rust runtime
    globalThis.__neo_tick_timers = () => {
        const now = Neo.time.now();

        // Process timeouts
        for (const [id, timer] of pendingTimers) {
            if (now >= timer.fireAt) {
                pendingTimers.delete(id);
                try {
                    timer.callback(...timer.args);
                } catch (e) {
                    Neo.log.error(`Timer error: ${e}`);
                }
            }
        }

        // Process intervals
        for (const [id, interval] of pendingIntervals) {
            if (now >= interval.nextFire) {
                interval.nextFire = now + interval.delay;
                try {
                    interval.callback(...interval.args);
                } catch (e) {
                    Neo.log.error(`Interval error: ${e}`);
                }
            }
        }
    };

    globalThis.setTimeout = (callback, delay = 0, ...args) => {
        const id = ++timerId;
        pendingTimers.set(id, {
            callback,
            args,
            fireAt: Neo.time.now() + delay,
        });
        return id;
    };

    globalThis.clearTimeout = (id) => {
        pendingTimers.delete(id);
    };

    globalThis.setInterval = (callback, delay = 0, ...args) => {
        const id = ++timerId;
        pendingIntervals.set(id, {
            callback,
            args,
            delay,
            nextFire: Neo.time.now() + delay,
        });
        return id;
    };

    globalThis.clearInterval = (id) => {
        pendingIntervals.delete(id);
    };

    // ─────────────────────────────────────────────────────────────────────────
    // Expose Neo globally
    // ─────────────────────────────────────────────────────────────────────────

    globalThis.Neo = Neo;

    Neo.log.debug("Neo plugin runtime initialized");

})(globalThis);
