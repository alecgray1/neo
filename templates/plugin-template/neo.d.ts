/**
 * Neo Plugin API Type Definitions
 */

declare global {
  /**
   * Neo global object available to all plugins
   */
  const Neo: {
    /**
     * Logging functions that send messages to the parent process
     */
    log: {
      trace(message: string): void;
      debug(message: string): void;
      info(message: string): void;
      warn(message: string): void;
      error(message: string): void;
    };

    /**
     * Event publishing
     */
    events: {
      /**
       * Publish an event to the event bus
       */
      publish(event: { type: string; data?: unknown }): void;
    };

    /**
     * Point read/write operations
     */
    points: {
      /**
       * Read a point value by ID
       */
      read(pointId: string): Promise<unknown>;

      /**
       * Write a value to a point by ID
       */
      write(pointId: string, value: unknown): Promise<void>;
    };

    /**
     * Plugin configuration (set by the host at startup)
     */
    config: Record<string, unknown>;
  };

  /**
   * Plugin lifecycle: Called when the plugin starts
   * @param payload - Startup payload including config
   */
  function onStart(payload?: { config?: Record<string, unknown> }): void | Promise<void>;

  /**
   * Plugin lifecycle: Called when the plugin stops
   */
  function onStop(): void | Promise<void>;

  /**
   * Plugin lifecycle: Called when the plugin receives an event
   * @param event - The event object
   */
  function onEvent(event: { type: string; source: string; data?: unknown }): void | Promise<void>;

  /**
   * Plugin lifecycle: Called on each tick interval (if configured)
   */
  function onTick(): void | Promise<void>;
}

export {};
