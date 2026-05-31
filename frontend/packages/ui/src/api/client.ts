import type { PlatformAPI } from '$lib/core/api.js';
import type { LoginRequest, Session } from '$lib/types/auth.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	Lorebook,
	LorebookMetadata,
	CreateLorebookRequest,
	LorebookEntry,
	UpdateEntryRequest,
	UpdateLorebookRequest
} from '$lib/types/lorebook.js';
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
			get: (id: Snowflake) => http.get<User>(`/api/v1/users/${id}`)
		},
		templates: {
			list: () => http.get<Array<TemplateMetadata>>('/api/v1/templates'),
			get: (id: Snowflake) => http.get<ContextTemplate>(`/api/v1/templates/${id}`),
			create: (create: CreateTemplateRequest) =>
				http.post<ContextTemplate>('/api/v1/templates', create),
			update: (id: Snowflake, update: UpdateTemplateRequest) =>
				http.put<ContextTemplate>(`/api/v1/templates/${id}`, update),
			delete: (id: Snowflake) => http.delete(`/api/v1/templates/${id}`),
			sources: () => http.get<Array<SourceDefinition>>('/api/v1/templates/sources'),
			nodes: {
				add: (templateId: Snowflake, create: CreateNodeRequest) =>
					http.post<NodeResponse>(`/api/v1/templates/${templateId}/nodes`, create),
				update: (templateId: Snowflake, nodeId: Snowflake, update: UpdateNodeRequest) =>
					http.put<NodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}`, update),
				delete: (templateId: Snowflake, nodeId: Snowflake) =>
					http.delete<StatusCodes>(`/api/v1/templates/${templateId}/nodes/${nodeId}`),
				move: (templateId: Snowflake, nodeId: Snowflake, move: MoveNodeRequest) =>
					http.post<MoveNodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}/move`, move),
				toggle: (templateId: Snowflake, nodeId: Snowflake) =>
					http.post<ToggleNodeResponse>(`/api/v1/templates/${templateId}/nodes/${nodeId}/toggle`)
			}
		},
		lorebooks: {
			list: () => http.get<Array<LorebookMetadata>>('/api/v1/lorebooks'),
			get: (id: Snowflake) => http.get<Lorebook>(`/api/v1/lorebooks/${id}`),
			create: (create: CreateLorebookRequest) => http.post<Lorebook>('/api/v1/lorebooks', create),
			update: (id: Snowflake, update: UpdateLorebookRequest) =>
				http.put<Lorebook>(`/api/v1/lorebooks/${id}`, update),
			delete: (id: Snowflake) => http.delete(`/api/v1/lorebooks/${id}`),
			entries: {
				get: (lorebookId: Snowflake, entryId: Snowflake) =>
					http.get<LorebookEntry>(`/api/v1/lorebooks/${lorebookId}/entries/${entryId}`),
				add: (lorebookId: Snowflake) =>
					http.post<LorebookEntry>(`/api/v1/lorebooks/${lorebookId}/entries`),
				update: (lorebookId: Snowflake, entryId: Snowflake, update: UpdateEntryRequest) =>
					http.put<LorebookEntry>(`/api/v1/lorebooks/${lorebookId}/entries/${entryId}`, update),
				delete: (lorebookId: Snowflake, entryId: Snowflake) =>
					http.delete(`/api/v1/lorebooks/${lorebookId}/entries/${entryId}`)
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
