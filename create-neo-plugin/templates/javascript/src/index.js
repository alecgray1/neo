// {{PLUGIN_NAME}} - Neo BMS Plugin

let intervalId = null;

defineService({
    async onStart() {
        const config = Neo.config;
        Neo.log.info('{{PLUGIN_NAME}} starting...');

        // {{INTERVAL_CODE}}

        Neo.log.info('{{PLUGIN_NAME}} started');
    },

    async onStop() {
        if (intervalId !== null) {
            clearInterval(intervalId);
            intervalId = null;
        }
        Neo.log.info('{{PLUGIN_NAME}} stopped');
    },

    async onEvent(event) {
        // {{EVENT_HANDLER}}
        Neo.log.debug(`Received event: ${event.type || 'unknown'}`);
    },

    async onRequest(request) {
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
