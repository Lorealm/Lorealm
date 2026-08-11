export { default as App } from './App.svelte';
export type {
	PlatformAPI,
	AuthApi,
	HttpTransport,
	LocalStore,
	ServerControls
} from './core/api.ts';
export type { User } from './types/user.js';
export type { Session } from './types/auth.ts';
export { extractError, errorDomain } from './types/error.js';
export type {
	ErrorCode,
	ErrorDomain,
	ErrorDetails,
	ExtractedError,
	ResourceKind
} from './types/error.js';
