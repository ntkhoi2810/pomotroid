import type { WeatherType } from '$lib/types';

const WEATHER_TYPES: WeatherType[] = ['sunny', 'rain', 'wind', 'storm'];

export function weatherForSession(sessionWorkCount: number): WeatherType {
  let seed = Math.max(1, sessionWorkCount) >>> 0;
  seed = Math.imul(seed ^ (seed >>> 16), 0x45d9f3b);
  seed = Math.imul(seed ^ (seed >>> 16), 0x45d9f3b);
  seed ^= seed >>> 16;
  return WEATHER_TYPES[(seed >>> 0) % WEATHER_TYPES.length];
}
