import { getContext, setContext } from 'svelte';
import type { PlatformAPI } from './api.ts';

const API_KEY = Symbol('my-app-api');

export function setAPI(api: PlatformAPI) {
	setContext(API_KEY, api);
}

export function getPlatformApi(): PlatformAPI {
	return getContext<PlatformAPI>(API_KEY);
}

export function hasCapability(cap: string): boolean {
	return getPlatformApi().capabilities.has(cap);
}
