// {{PLUGIN_NAME}} - Neo BMS Plugin (TypeScript)

// Type definitions for Neo SDK
declare const Neo: {
    readonly pluginId: string;
    readonly config: Record<string, unknown>;
    points: {
        read(path: string): Promise<PointValue>;
        write(path: string, value: PointValue): void;
    };
    events: {
        publish(event: NeoEvent): void;
    };
    log: {
        trace(msg: string): void;
        debug(msg: string): void;
        info(msg: string): void;
        warn(msg: string): void;
        error(msg: string): void;
    };
    time: {
        now(): number;
    };
};

declare function defineService(plugin: ServicePlugin): void;

interface ServicePlugin {
    onStart?(): Promise<void>;
    onStop?(): Promise<void>;
    onEvent?(event: NeoEvent): Promise<void>;
    onRequest?(request: ServiceRequest): Promise<ServiceResponse>;
}

type PointValue =
    | { Null: null }
    | { Boolean: boolean }
    | { Real: number }
    | { Integer: number }
    | { String: string }
    | { Enum: number }
    | { Array: PointValue[] };

interface NeoEvent {
    type: string;
    source?: string;
    data?: unknown;
    [key: string]: unknown;
}

interface ServiceRequest {
    type: string;
    action?: string;
    data?: unknown;
}

type ServiceResponse =
    | { type: 'Ok' }
    | { type: 'Custom'; data: unknown }
    | { type: 'Error'; code: string; message: string };

// Plugin implementation
let intervalId: number | null = null;

defineService({
    async onStart(): Promise<void> {
        const config = Neo.config;
        Neo.log.info('{{PLUGIN_NAME}} starting...');

        // {{INTERVAL_CODE}}

        Neo.log.info('{{PLUGIN_NAME}} started');
    },

    async onStop(): Promise<void> {
        if (intervalId !== null) {
            clearInterval(intervalId);
            intervalId = null;
        }
        Neo.log.info('{{PLUGIN_NAME}} stopped');
    },

    async onEvent(event: NeoEvent): Promise<void> {
        // {{EVENT_HANDLER}}
        Neo.log.debug(`Received event: ${event.type || 'unknown'}`);
    },

    async onRequest(request: ServiceRequest): Promise<ServiceResponse> {
        if (request.type === 'Custom') {
            const action = request.action;

            if (action === 'getStatus') {
                return {
                    type: 'Custom',
                    data: {
                        status: 'ok',
                        timestamp: Neo.time.now(),
                    },
                };
            }

            // Add more custom actions here
        }

        return {
            type: 'Error',
            code: 'UNKNOWN_REQUEST',
            message: `Unknown request: ${request.type}`,
        };
    },
});

// {{HELPER_FUNCTIONS}}
