// Weather Service Plugin
// Demonstrates a Neo plugin that polls external data and updates virtual points

import {
    defineService,
    getConfig,
    real,
    unsigned,
    okResponse,
    customResponse,
    errorResponse,
    type ServiceRequest,
    type ServiceResponse,
    type NeoEvent,
    type JsonValue,
} from "../../sdk/src/index.ts";

interface WeatherConfig {
    api_key: string;
    location: string;
    poll_interval_seconds: number;
    units: "imperial" | "metric";
}

interface WeatherData {
    temperature: number;
    humidity: number;
    conditions: string;
    description: string;
    wind_speed: number;
    pressure: number;
}

// Cached weather data
let cachedWeather: WeatherData | null = null;
let pollIntervalId: number | null = null;

defineService({
    async onStart() {
        const config = getConfig<WeatherConfig>();
        Neo.log.info(`Weather service starting for ${config.location}`);

        // Initial fetch
        await fetchAndUpdateWeather(config);

        // Start polling
        pollIntervalId = setInterval(
            () => fetchAndUpdateWeather(config),
            config.poll_interval_seconds * 1000
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

    async onEvent(_event: NeoEvent) {
        // This plugin doesn't subscribe to any events
    },

    async onRequest(request: ServiceRequest): Promise<ServiceResponse> {
        if (request.type === "Custom") {
            const action = request.action;

            if (action === "refresh") {
                const config = getConfig<WeatherConfig>();
                await fetchAndUpdateWeather(config);
                return okResponse();
            }

            if (action === "getWeather") {
                if (cachedWeather) {
                    return customResponse(cachedWeather as unknown as JsonValue);
                }
                return errorResponse("NO_DATA", "No weather data available yet");
            }

            if (action === "getLocation") {
                const config = getConfig<WeatherConfig>();
                return customResponse({ location: config.location } as JsonValue);
            }
        }

        return errorResponse("UNKNOWN_REQUEST", `Unknown request type: ${request.type}`);
    },
});

async function fetchAndUpdateWeather(config: WeatherConfig) {
    try {
        const weather = await fetchWeatherData(config);
        cachedWeather = weather;

        // Write to virtual points
        Neo.points.write("virtual/weather/outdoor_temp", real(weather.temperature));
        Neo.points.write("virtual/weather/humidity", real(weather.humidity));
        Neo.points.write("virtual/weather/wind_speed", real(weather.wind_speed));
        Neo.points.write("virtual/weather/pressure", real(weather.pressure));
        Neo.points.write("virtual/weather/conditions", unsigned(weatherCodeFromConditions(weather.conditions)));

        // Publish weather update event
        Neo.events.publish({
            type: "WeatherUpdated",
            source: "weather-service",
            timestamp: new Date().toISOString(),
            data: weather,
        });

        Neo.log.info(`Weather updated: ${weather.temperature}°, ${weather.conditions}`);
    } catch (error) {
        Neo.log.error(`Failed to fetch weather: ${error}`);
    }
}

async function fetchWeatherData(config: WeatherConfig): Promise<WeatherData> {
    // In a full implementation, this would call an actual weather API
    // For now, we simulate weather data based on the location hash
    //
    // To use a real API like OpenWeatherMap:
    // const response = await fetch(
    //     `https://api.openweathermap.org/data/2.5/weather?q=${encodeURIComponent(config.location)}&appid=${config.api_key}&units=${config.units}`
    // );
    // const data = await response.json();
    // return {
    //     temperature: data.main.temp,
    //     humidity: data.main.humidity,
    //     conditions: data.weather[0].main,
    //     description: data.weather[0].description,
    //     wind_speed: data.wind.speed,
    //     pressure: data.main.pressure,
    // };

    // Simulated weather data
    const now = Neo.time.now();
    const hourOfDay = Math.floor((now / 3600000) % 24);
    const locationHash = hashCode(config.location);

    // Base temperature varies by "location" and time of day
    let baseTemp = 60 + (locationHash % 30);
    // Add daily variation (warmer midday, cooler night)
    baseTemp += Math.sin((hourOfDay - 6) * Math.PI / 12) * 15;

    // Convert if metric
    const temperature = config.units === "metric"
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

function hashCode(str: string): number {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
        const char = str.charCodeAt(i);
        hash = ((hash << 5) - hash) + char;
        hash = hash & hash; // Convert to 32bit integer
    }
    return Math.abs(hash);
}

function getConditionsForHash(hash: number): string {
    const conditions = ["Clear", "Clouds", "Partly Cloudy", "Rain", "Drizzle"];
    return conditions[hash % conditions.length];
}

function weatherCodeFromConditions(conditions: string): number {
    const codes: Record<string, number> = {
        "Clear": 0,
        "Clouds": 1,
        "Partly Cloudy": 2,
        "Rain": 3,
        "Drizzle": 4,
        "Thunderstorm": 5,
        "Snow": 6,
        "Mist": 7,
        "Fog": 8,
    };
    return codes[conditions] ?? 255;
}
