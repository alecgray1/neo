// Random Number Service Plugin
// Generates a random number at a configurable interval

let intervalId = null;
let currentValue = null;

defineService({
    async onStart() {
        const config = Neo.config;
        const min = config.min ?? 0;
        const max = config.max ?? 100;
        const intervalSecs = config.interval_seconds ?? 5;

        Neo.log.info(`Random service starting (range: ${min}-${max}, interval: ${intervalSecs}s)`);

        // Generate initial value
        generateRandom(min, max);

        // Start interval
        intervalId = setInterval(() => {
            generateRandom(min, max);
        }, intervalSecs * 1000);

        Neo.log.info("Random service started");
    },

    async onStop() {
        if (intervalId !== null) {
            clearInterval(intervalId);
            intervalId = null;
        }
        currentValue = null;
        Neo.log.info("Random service stopped");
    },

    async onEvent(event) {
        // This plugin doesn't process events
    },

    async onRequest(request) {
        if (request.type === "Custom") {
            if (request.action === "getValue") {
                return {
                    type: "Custom",
                    data: { value: currentValue }
                };
            }

            if (request.action === "generate") {
                const config = Neo.config;
                generateRandom(config.min ?? 0, config.max ?? 100);
                return {
                    type: "Custom",
                    data: { value: currentValue }
                };
            }
        }

        return {
            type: "Error",
            code: "UNKNOWN_REQUEST",
            message: `Unknown request: ${request.type}`
        };
    },
});

function generateRandom(min, max) {
    currentValue = Math.floor(Math.random() * (max - min + 1)) + min;

    // Write to virtual point
    Neo.points.write("virtual/random/value", { Real: currentValue });

    // Publish event
    Neo.events.publish({
        type: "RandomValueGenerated",
        source: "random-service",
        data: { value: currentValue, min, max }
    });

    Neo.log.info(`Generated random number: ${currentValue}`);
}
