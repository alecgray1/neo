// Weather Service Plugin (JavaScript)
// Demonstrates a Neo plugin that provides simulated weather data

let cachedWeather = null;
let pollIntervalId = null;

defineService({
    async onStart() {
        const config = Neo.config;
        Neo.log.info(`Weather service starting for ${config.location || 'Unknown'}`);

        // Initial fetch
        await fetchAndUpdateWeather(config);

        // Start polling (every 30 seconds for demo)
        const pollInterval = (config.poll_interval_seconds || 30) * 1000;
        pollIntervalId = setInterval(
            () => fetchAndUpdateWeather(config),
            pollInterval
        );

        Neo.log.info("Weather service started successfully");
    },

    async onStop() {
        if (pollIntervalId !== null) {
            clearInterval(pollIntervalId);
            pollIntervalId = null;
        }
        cachedWeather = null;
        Neo.log.info("Weather service stopped");
    },

    async onEvent(event) {
        // Log events we receive (for debugging)
        Neo.log.debug(`Weather service received event: ${event.type || 'unknown'}`);
    },

    async onRequest(request) {
        if (request.type === "Custom") {
            const action = request.action;

            if (action === "refresh") {
                await fetchAndUpdateWeather(Neo.config);
                return { type: "Ok" };
            }

            if (action === "getWeather") {
                if (cachedWeather) {
                    return {
                        type: "Custom",
                        data: cachedWeather
                    };
                }
                return {
                    type: "Error",
                    code: "NO_DATA",
                    message: "No weather data available yet"
                };
            }

            if (action === "getLocation") {
                return {
                    type: "Custom",
                    data: { location: Neo.config.location || 'Unknown' }
                };
            }
        }

        return {
            type: "Error",
            code: "UNKNOWN_REQUEST",
            message: `Unknown request type: ${request.type}`
        };
    },
});

async function fetchAndUpdateWeather(config) {
    try {
        const weather = await fetchWeatherData(config);
        cachedWeather = weather;

        // Write to virtual points
        Neo.points.write("virtual/weather/outdoor_temp", { Real: weather.temperature });
        Neo.points.write("virtual/weather/humidity", { Real: weather.humidity });
        Neo.points.write("virtual/weather/wind_speed", { Real: weather.wind_speed });
        Neo.points.write("virtual/weather/pressure", { Real: weather.pressure });

        // Publish weather update event
        Neo.events.publish({
            type: "WeatherUpdated",
            source: "weather-service",
            data: weather,
        });

        Neo.log.info(`Weather updated: ${weather.temperature}°, ${weather.conditions}`);
    } catch (error) {
        Neo.log.error(`Failed to fetch weather: ${error}`);
    }
}

async function fetchWeatherData(config) {
    // Simulated weather data
    const now = Neo.time.now();
    const hourOfDay = Math.floor((now / 3600000) % 24);
    const locationHash = hashCode(config.location || 'Unknown');

    // Base temperature varies by "location" and time of day
    let baseTemp = 60 + (locationHash % 30);
    // Add daily variation (warmer midday, cooler night)
    baseTemp += Math.sin((hourOfDay - 6) * Math.PI / 12) * 15;

    // Convert if metric
    const units = config.units || 'imperial';
    const temperature = units === "metric"
        ? (baseTemp - 32) * 5 / 9
        : baseTemp;

    const conditions = getConditionsForHash(locationHash + Math.floor(now / 86400000));

    return {
        temperature: Math.round(temperature * 10) / 10,
        humidity: 40 + (locationHash % 40),
        conditions,
        description: conditions.toLowerCase(),
        wind_speed: 5 + (locationHash % 15),
        pressure: 1000 + (locationHash % 30),
    };
}

function hashCode(str) {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash;
    }
    return Math.abs(hash);
}

function getConditionsForHash(hash) {
    const conditions = ["Clear", "Clouds", "Partly Cloudy", "Rain", "Drizzle"];
    return conditions[hash % conditions.length];
}
