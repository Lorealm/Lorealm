import type { PlatformAPI } from '$lib/core/api.js';
import type { LoginRequest, Session } from '$lib/types/auth.js';
import type {
	Character,
	CharacterEdit,
	CharacterSummary,
	CharacterVersionInfo,
	CreateCharacterRequest,
	ImportResult
} from '$lib/types/character.js';
import type {
	ConversationMetadata,
	ChatEdit,
	ChatStreamEvent,
	CompletionsRequest,
	ConversationMessage,
	FullConversation,
	StartChatRequest
} from '$lib/types/conversation.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	Lorebook,
	LorebookMetadata,
	CreateLorebookRequest,
	LorebookEntry,
	UpdateEntryRequest,
	UpdateLorebookRequest,
	SetActiveLorebooksRequest
} from '$lib/types/lorebook.js';
import type { SourceDefinition } from '$lib/types/node.js';
import type { ReconstructedState } from '$lib/types/state.js';
import type {
	ContextTemplate,
	CreateNodeRequest,
	CreateTemplateRequest,
	MoveNodeRequest,
	MoveNodeResponse,
	NodeResponse,
	SetActiveRequest as SetActiveTemplateRequest,
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
			active: {
				get: () => http.get<Snowflake | null>('/api/v1/templates/active'),
				set: (request: SetActiveTemplateRequest) => http.put('/api/v1/templates/active', request),
				clear: () => http.delete('/api/v1/templates/active')
			},
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
			active: {
				get: () => http.get<Array<Snowflake> | null>('/api/v1/lorebooks/active'),
				set: (request: SetActiveLorebooksRequest) => http.put('/api/v1/lorebooks/active', request)
			},
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
		},
		characters: {
			list: () => http.get<Array<CharacterSummary>>('/api/v1/characters'),
			get: (characterId: Snowflake) => http.get<Character>(`/api/v1/characters/${characterId}`),
			create: (create: CreateCharacterRequest) =>
				http.post<Character>('/api/v1/characters', create),
			/** __Note on edit:__ Edits are applied in the same order the array is constructed
			 *
			 * The request will fail if any given edits fail, but preceeding edits will still apply.
			 * Make sure to account for this on failure by refetching cards from source accordingly.
			 */
			update: (characterId: Snowflake, edit: CharacterEdit[]) =>
				http.put<Character>(`/api/v1/characters/${characterId}`, edit),
			avatar: (characterId: Snowflake) => http.getBlob(`/api/v1/characters/${characterId}/image`),
			import: () =>
				http.post<ImportResult>(`/api/v1/characters/import`, {
					placeholder: 'this endpoint is not valid yet'
				}),
			chats: (characterId: Snowflake) =>
				http.get<Array<ConversationMetadata>>(`/api/v1/characters/${characterId}/chats`),
			versions: {
				list: (characterId: Snowflake) =>
					http.get<Array<CharacterVersionInfo>>(`/api/v1/characters/${characterId}/versions`),
				get: (characterId: Snowflake, versionId: Snowflake) =>
					http.get<Character>(`/api/v1/characters/${characterId}/versions/${versionId}`),
				avatar: (characterId: Snowflake, versionId: Snowflake) =>
					http.get(`/api/v1/characters/${characterId}/versions/${versionId}/image`)
			}
		},
		chat: {
			list: () => http.get<Array<ConversationMetadata>>('/api/v1/chat'),
			start: (request: StartChatRequest) => http.post<FullConversation>('/api/v1/chat', request),
			get: (conversation_id: Snowflake, full: boolean) =>
				http.get<FullConversation>(`/api/v1/chat/${conversation_id}?full=${full}`),
			delete: (conversation_id: Snowflake) => http.delete(`/api/v1/chat/${conversation_id}`),
			completions: (conversation_id: Snowflake, request: CompletionsRequest) =>
				http.stream<ChatStreamEvent>(`/api/v1/chat/${conversation_id}/completions`, request),
			messages: {
				list: (conversation_id: Snowflake) =>
					http.get<Array<ConversationMessage>>(`/api/v1/chat/${conversation_id}/messages`),
				get: (conversation_id: Snowflake, message_id: Snowflake) =>
					http.get<ConversationMessage>(`/api/v1/chat/${conversation_id}/messages/${message_id}`),
				edit: (conversation_id: Snowflake, edit: ChatEdit) =>
					http.put(`/api/v1/chat/${conversation_id}/messages`, edit),
				delete: (conversation_id: Snowflake, message_id: Snowflake) =>
					http.delete(`/api/v1/chat/${conversation_id}/messages/${message_id}`),
				state: (conversation_id: Snowflake, message_id: Snowflake) =>
					http.get<ReconstructedState>(
						`/api/v1/chat/${conversation_id}/messages/${message_id}/state`
					)
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
