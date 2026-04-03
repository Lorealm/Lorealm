import type { PlatformAPI } from '$lib/core/api.js';
import type { LoginRequest, Session } from '$lib/types/auth.js';
import type { User } from '$lib/types/user.js';
import { getContext, setContext } from 'svelte';
const API_KEY = Symbol('api-client');

export function createApiClient(platform: PlatformAPI) {
	const { http } = platform;

	return {
		auth: {
			login: (login: LoginRequest) => http.post<Session>('/api/v1/login', login)
		},
		users: {
			me: () => http.get<User>('/api/v1/users/me'),
			get: (id: string) => http.get<User>(`/api/v1/users/${id}`)
		}
	};
}

export type ApiClient = ReturnType<typeof createApiClient>;

export function provideApiClient(platform: PlatformAPI): ApiClient {
	const client = createApiClient(platform);
	setContext(API_KEY, client);
	return client;
}

export function useApi(): ApiClient {
	return getContext<ApiClient>(API_KEY);
}
