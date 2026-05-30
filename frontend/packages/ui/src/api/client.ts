import type { PlatformAPI } from '$lib/core/api.js';
import type { LoginRequest, Session } from '$lib/types/auth.js';
import type { SourceDefinition } from '$lib/types/node.js';
import type {
	ContextTemplate,
	CreateNodeRequest,
	CreateTemplateRequest,
	MoveNodeRequest,
	MoveNodeResponse,
	NodeResponse,
	TemplateMetadata,
	ToggleNodeResponse,
	UpdateNodeRequest,
	UpdateTemplateRequest
} from '$lib/types/template.js';
import type { User } from '$lib/types/user.js';
import type { StatusCodes } from 'http-status-codes';
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
		},
		templates: {
			list: () => http.get<Array<TemplateMetadata>>('/api/v1/templates'),
			get: (id: string) => http.get<ContextTemplate>(`/api/v1/templates/${id}`),
			create: (create: CreateTemplateRequest) =>
				http.post<ContextTemplate>('/api/v1/templates', create),
			update: (id: string, update: UpdateTemplateRequest) =>
				http.put<ContextTemplate>(`/api/v1/templates/${id}`, update),
			delete: (id: string) => http.delete(`/api/v1/templates/${id}`),
			sources: () => http.get<Array<SourceDefinition>>('/api/v1/templates/sources'),
			nodes: {
				add: (templateId: string, create: CreateNodeRequest) =>
					http.post<NodeResponse>(`/api/v1/templates/${templateId}/nodes`, create),
				update: (templateId: string, nodeId: string, update: UpdateNodeRequest) =>
					http.put<NodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}`, update),
				delete: (templateId: string, nodeId: string) =>
					http.delete<StatusCodes>(`/api/v1/templates/${templateId}/nodes/${nodeId}`),
				move: (templateId: string, nodeId: string, move: MoveNodeRequest) =>
					http.post<MoveNodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}/move`, move),
				toggle: (templateId: string, nodeId: string) =>
					http.post<ToggleNodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}/toggle`)
			}
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
