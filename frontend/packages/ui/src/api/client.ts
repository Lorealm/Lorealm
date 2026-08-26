import type { PlatformAPI } from '$lib/core/api.js';
import type { LoginRequest, Session } from '$lib/types/auth.js';
import type {
	AvatarUpdateResponse,
	Character,
	CharacterEdit,
	CharacterSummary,
	CharacterVersionInfo,
	CreateCharacterRequest,
	ImportCharResponse
} from '$lib/types/character.js';
import type { ConfigKey, ServerConfig } from '$lib/types/config.js';
import type {
	ConnectionProfile,
	CreateSecretRequest,
	DeleteSecretResponse,
	ProfileRequest,
	RenameSecretRequest,
	SecretMetadata,
	SetActiveProfileRequest
} from '$lib/types/connection.js';
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
import type { ProviderModel } from '$lib/types/schema.js';
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
import type { WeaverVocabulary } from '$lib/types/weaver.js';
import type {
	UserPersona,
	UpdatePersona,
	PersonaOverride,
	SetPersonaOverride
} from '$lib/types/persona.js';
import type {
	ActionSpec,
	AgentGrant,
	AgentObservation,
	CreateAgentRequest,
	CreateRealmRequest,
	CreateScenarioRequest,
	ImportRule,
	NextTurn,
	PlayerTurnRequest,
	Realm,
	RealmAgent,
	RealmCast,
	RealmConfig,
	RealmEvent,
	RealmStreamEvent,
	RealmSummary,
	RealmVersionInfo,
	RunTurnRequest,
	Scenario,
	StartRealmChatRequest,
	StateSlice,
	TurnResult,
	UpdateRealmRequest
} from '$lib/types/realm.js';
import type { ReconstructedState } from '$lib/types/state.js';
import type {
	ContextTemplate,
	CreateNodeRequest,
	CreateTemplateRequest,
	MoveNodeRequest,
	MoveNodeResponse,
	NodeResponse,
	SetActiveRequest,
	SetActiveRequest as SetActiveTemplateRequest,
	TemplateMetadata,
	ToggleNodeResponse,
	UpdateNodeRequest,
	UpdateTemplateRequest
} from '$lib/types/template.js';
import type {
	BatchTokenizeRequest,
	BatchTokenizeResponse,
	NetworkCreate,
	ShareRequest,
	Tokenizer,
	TokenizeRequest,
	TokenizerShare
} from '$lib/types/tokenizer.js';
import type {
	CreateUserTool,
	ToolEnabledRequest,
	UpdateUserTool,
	UserTool
} from '$lib/types/tools.js';
import type {
	CreateUserWidgetRequest,
	UpdateUserWidgetRequest,
	UserWidget
} from '$lib/types/widgets.js';
import type { User } from '$lib/types/user.js';
import type { StatusCodes } from 'http-status-codes';

/**
 * Encode cards as a `multipart/form-data` body.
 * 
 * NOTE: This function was written with the help of AI
 */
function encodeCardBatch(cards: Blob[]): Blob {
	const boundary = `----lorealmcards${Math.random().toString(36).slice(2)}${Date.now().toString(36)}`;
	const parts: BlobPart[] = [];

	cards.forEach((card, i) => {
		// The server skips fields without a filename, so every part needs one.
		const name = (card instanceof File ? card.name : `card-${i}`).replace(/["\r\n]/g, '_');
		parts.push(
			`--${boundary}\r\n` +
				`Content-Disposition: form-data; name="files"; filename="${name}"\r\n` +
				`Content-Type: ${card.type || 'application/octet-stream'}\r\n\r\n`
		);
		parts.push(card);
		parts.push('\r\n');
	});
	parts.push(`--${boundary}--\r\n`);

	return new Blob(parts, { type: `multipart/form-data; boundary=${boundary}` });
}

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
			weaverVocabulary: () => http.get<WeaverVocabulary>('/api/v1/templates/weaver-vocabulary'),
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
			updateAvatar: (characterId: Snowflake, update: Blob) =>
				http.putBlob<AvatarUpdateResponse>(`/api/v1/characters/${characterId}/image`, update),
			import: (card: Blob) => http.putBlob<ImportCharResponse>(`/api/v1/characters/import`, card),
			/** Imports every card in one request. Results come back in the order given. */
			importBatch: (cards: Blob[]) =>
				http.putBlob<ImportCharResponse>(`/api/v1/characters/importBatch`, encodeCardBatch(cards)),
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
			cancel: (conversation_id: Snowflake) =>
				http.delete(`/api/v1/chat/${conversation_id}/completions`),
			realm: {
				/** Instantiate a realm preset into a fresh playthrough conversation. */
				start: (request: StartRealmChatRequest) =>
					http.post<FullConversation>('/api/v1/chat/realm', request),
				/** The cast snapshot pinned when this playthrough started. */
				cast: (conversation_id: Snowflake) =>
					http.get<RealmCast>(`/api/v1/chat/${conversation_id}/cast`),
				nextTurn: (conversation_id: Snowflake) =>
					http.get<NextTurn>(`/api/v1/chat/${conversation_id}/turn/next`),
				/** The playthrough's objective timeline (id-paged backward). */
				events: (conversation_id: Snowflake, before?: Snowflake) =>
					http.get<Array<RealmEvent>>(
						`/api/v1/chat/${conversation_id}/events${before !== undefined ? `?before=${before}` : ''}`
					),
				/** Run the next LLM turn (or regenerate `target`) and stream it. */
				turnStream: (conversation_id: Snowflake, request: RunTurnRequest = {}) =>
					http.stream<RealmStreamEvent>(`/api/v1/chat/${conversation_id}/turn/stream`, request),
				/** Submit the current player's turn (text and/or one bound action). */
				playerTurn: (conversation_id: Snowflake, turn: PlayerTurnRequest) =>
					http.post<TurnResult>(`/api/v1/chat/${conversation_id}/turn`, turn),
				agents: {
					observations: (conversation_id: Snowflake, agent_id: Snowflake, before?: Snowflake) =>
						http.get<Array<AgentObservation>>(
							`/api/v1/chat/${conversation_id}/agents/${agent_id}/observations${before !== undefined ? `?before=${before}` : ''}`
						),
					actions: (conversation_id: Snowflake, agent_id: Snowflake) =>
						http.get<Array<ActionSpec>>(
							`/api/v1/chat/${conversation_id}/agents/${agent_id}/actions`
						),
					/** The agent's granted slice: at `at` when given, else the active leaf. */
					state: (conversation_id: Snowflake, agent_id: Snowflake, at?: Snowflake) =>
						http.get<StateSlice>(
							`/api/v1/chat/${conversation_id}/agents/${agent_id}/state${at !== undefined ? `?at=${at}` : ''}`
						)
				}
			},
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
		},
		personas: {
			list: () => http.get<Array<UserPersona>>('/api/v1/persona'),
			create: () => http.post<UserPersona>('/api/v1/persona'),
			update: (persona_id: Snowflake, update: UpdatePersona) =>
				http.put<UserPersona>(`/api/v1/persona/${persona_id}`, update),
			setAvatar: (persona_id: Snowflake, avatar: Blob) =>
				http.putBlob(`/api/v1/persona/${persona_id}/avatar`, avatar),
			active: {
				get: () => http.get<Snowflake | null>('/api/v1/persona/active'),
				set: (request: SetActiveRequest) => http.put('/api/v1/persona/active', request)
			},
			/** Per-character persona bindings, which win over the active persona. */
			overrides: {
				list: () => http.get<Array<PersonaOverride>>('/api/v1/persona/overrides'),
				set: (character_id: Snowflake, request: SetPersonaOverride) =>
					http.put<PersonaOverride>(`/api/v1/persona/overrides/${character_id}`, request),
				clear: (character_id: Snowflake) => http.delete(`/api/v1/persona/overrides/${character_id}`)
			}
		},
		connection: {
			secrets: {
				list: () => http.get<Array<SecretMetadata>>('/api/v1/connections/secrets'),
				create: (create: CreateSecretRequest) =>
					http.post<SecretMetadata>('/api/v1/connections/secrets', create),
				rename: (secret_id: Snowflake, rename: RenameSecretRequest) =>
					http.put<SecretMetadata>(`/api/v1/connections/secrets/${secret_id}`, rename),
				delete: (secret_id: Snowflake) =>
					http.delete<DeleteSecretResponse>(`/api/v1/connections/secrets/${secret_id}`)
			},
			profiles: {
				list: () => http.get<Array<ConnectionProfile>>('/api/v1/connections/profiles'),
				create: (create: ProfileRequest) =>
					http.post<ConnectionProfile>('/api/v1/connections/profiles', create),
				get: (profile_id: Snowflake) =>
					http.get<ConnectionProfile>(`/api/v1/connections/profiles/${profile_id}`),
				update: (profile_id: Snowflake, update: ProfileRequest) =>
					http.put<ConnectionProfile>(`/api/v1/connections/profiles/${profile_id}`, update),
				delete: (profile_id: Snowflake) =>
					http.delete(`/api/v1/connections/profiles/${profile_id}`),
				models: (profile_id: Snowflake) =>
					http.get<Array<ProviderModel>>(`/api/v1/connections/profiles/${profile_id}/models`),
				active: {
					get: () => http.get<Snowflake>('/api/v1/connections/profiles/active'),
					set: (request: SetActiveProfileRequest) =>
						http.post('/api/v1/connections/profiles/active', request)
				}
			}
		},
		tools: {
			list: () => http.get<Array<UserTool>>('/api/v1/tools'),
			builtins: () => http.get<Array<UserTool>>('/api/v1/tools/builtin'),
			get: (tool_id: Snowflake) => http.get<UserTool>(`/api/v1/tools/${tool_id}`),
			create: (create: CreateUserTool) => http.post<Snowflake>('/api/v1/tools', create),
			update: (tool_id: Snowflake, update: UpdateUserTool) =>
				http.put<UserTool>(`/api/v1/tools/${tool_id}`, update),
			toggle: (tool_id: Snowflake, enable: ToolEnabledRequest) =>
				http.patch(`/api/v1/tools/${tool_id}`, enable),
			delete: (tool_id: Snowflake) => http.delete(`/api/v1/tools/${tool_id}`)
		},
		userWidgets: {
			list: () => http.get<Array<UserWidget>>('/api/v1/widgets'),
			get: (widget_id: Snowflake) => http.get<UserWidget>(`/api/v1/widgets/${widget_id}`),
			create: (create: CreateUserWidgetRequest) => http.post<UserWidget>('/api/v1/widgets', create),
			update: (widget_id: Snowflake, update: UpdateUserWidgetRequest) =>
				http.put<UserWidget>(`/api/v1/widgets/${widget_id}`, update),
			toggle: (widget_id: Snowflake, enabled: boolean) =>
				http.patch(`/api/v1/widgets/${widget_id}`, { enabled }),
			delete: (widget_id: Snowflake) => http.delete(`/api/v1/widgets/${widget_id}`)
		},
		tokenizers: {
			list: () => http.get<Array<Tokenizer>>('/api/v1/tokenizers'),
			get: (tok_id: Snowflake) => http.get<Tokenizer>(`/api/v1/tokenizers/${tok_id}`),
			delete: (tok_id: Snowflake) => http.delete(`/api/v1/tokenizers/${tok_id}`),
			count: {
				single: (tok_id: Snowflake, req: TokenizeRequest) =>
					http.post<number>(`/api/v1/tokenizers/${tok_id}/count`, req),
				batch: (tok_id: Snowflake, req: BatchTokenizeRequest) =>
					http.post<BatchTokenizeResponse>(`/api/v1/tokenizers/${tok_id}/batchCount`, req)
			},
			shares: {
				list: (tok_id: Snowflake) =>
					http.get<Array<TokenizerShare>>(`/api/v1/tokenizers/${tok_id}/shares`),
				share: (tok_id: Snowflake, share: ShareRequest) =>
					http.post(`/api/v1/tokenizers/${tok_id}/shares`, share),
				unshare: (tok_id: Snowflake, user_id: Snowflake) =>
					http.delete(`/api/v1/tokenizers/${tok_id}/shares/${user_id}`)
			},
			imports: {
				file: (blob: Blob) => http.putBlob<Tokenizer>('/api/v1/tokenizers/import', blob),
				network: (res: NetworkCreate) => http.post<Tokenizer>('/api/v1/tokenizers/network', res)
			},
			active: {
				get: () => http.get<Snowflake | null>('/api/v1/tokenizers/active'),
				set: (active: SetActiveRequest) => http.put('/api/v1/tokenizers/active', active),
				unset: () => http.delete('/api/v1/tokenizers/active')
			}
		},
		realms: {
			list: () => http.get<Array<RealmSummary>>('/api/v1/realms'),
			get: (realm_id: Snowflake) => http.get<Realm>(`/api/v1/realms/${realm_id}`),
			create: (create: CreateRealmRequest) => http.post<Realm>('/api/v1/realms', create),
			update: (realm_id: Snowflake, update: UpdateRealmRequest) =>
				http.patch<Realm>(`/api/v1/realms/${realm_id}`, update),
			delete: (realm_id: Snowflake) => http.delete(`/api/v1/realms/${realm_id}`),
			imports: {
				/** Full-replace of the realm's import manifest. */
				set: (realm_id: Snowflake, imports: ImportRule[]) =>
					http.put<Array<ImportRule>>(`/api/v1/realms/${realm_id}/imports`, imports)
			},
			/** A realm's formatting overrides. */
			config: {
				get: (realm_id: Snowflake) => http.get<RealmConfig>(`/api/v1/realms/${realm_id}/config`),
				set: (realm_id: Snowflake, config: RealmConfig) =>
					http.put<RealmConfig>(`/api/v1/realms/${realm_id}/config`, config)
			},
			versions: {
				list: (realm_id: Snowflake) =>
					http.get<Array<RealmVersionInfo>>(`/api/v1/realms/${realm_id}/versions`),
				get: (realm_id: Snowflake, version_id: Snowflake) =>
					http.get<Realm>(`/api/v1/realms/${realm_id}/versions/${version_id}`),
				restore: (realm_id: Snowflake, version_id: Snowflake) =>
					http.post<Realm>(`/api/v1/realms/${realm_id}/versions/${version_id}/restore`)
			},
			agents: {
				list: (realm_id: Snowflake) =>
					http.get<Array<RealmAgent>>(`/api/v1/realms/${realm_id}/agents`),
				create: (realm_id: Snowflake, create: CreateAgentRequest) =>
					http.post<RealmAgent>(`/api/v1/realms/${realm_id}/agents`, create),
				update: (realm_id: Snowflake, agent_id: Snowflake, update: CreateAgentRequest) =>
					http.put<RealmAgent>(`/api/v1/realms/${realm_id}/agents/${agent_id}`, update),
				/** Rewrite the turn order; must be a permutation of the cast's ids. */
				reorder: (realm_id: Snowflake, order: Snowflake[]) =>
					http.put<Array<RealmAgent>>(`/api/v1/realms/${realm_id}/agents/order`, order),
				delete: (realm_id: Snowflake, agent_id: Snowflake) =>
					http.delete(`/api/v1/realms/${realm_id}/agents/${agent_id}`),
				grants: {
					get: (realm_id: Snowflake, agent_id: Snowflake) =>
						http.get<Array<AgentGrant>>(`/api/v1/realms/${realm_id}/agents/${agent_id}/grants`),
					set: (realm_id: Snowflake, agent_id: Snowflake, grants: AgentGrant[]) =>
						http.put(`/api/v1/realms/${realm_id}/agents/${agent_id}/grants`, grants)
				},
				/** The agent's action space as currently authored in the preset. */
				actions: (realm_id: Snowflake, agent_id: Snowflake) =>
					http.get<Array<ActionSpec>>(`/api/v1/realms/${realm_id}/agents/${agent_id}/actions`)
			},
			scenarios: {
				create: (realm_id: Snowflake, create: CreateScenarioRequest) =>
					http.post<Scenario>(`/api/v1/realms/${realm_id}/scenarios`, create),
				update: (realm_id: Snowflake, scenario_id: Snowflake, update: CreateScenarioRequest) =>
					http.put<Scenario>(`/api/v1/realms/${realm_id}/scenarios/${scenario_id}`, update),
				delete: (realm_id: Snowflake, scenario_id: Snowflake) =>
					http.delete(`/api/v1/realms/${realm_id}/scenarios/${scenario_id}`)
			}
		},
		config: {
			getAll: () => http.get<ServerConfig>('/api/v1/config'),
			get: <K extends ConfigKey>(key: K) => http.get<ServerConfig[K]>(`/api/v1/config/${key}`),
			patch: <K extends ConfigKey>(key: K, partial: ServerConfig[K]) =>
				http.patch<ServerConfig[K]>(`/api/v1/config/${key}`, partial)
		}
	};
}

export type ApiClient = ReturnType<typeof createApiClient>;

let client: ApiClient | null = null;
let platformApi: PlatformAPI | null = null;

export function provideApiClient(platform: PlatformAPI): ApiClient {
	platformApi = platform;
	client = createApiClient(platform);
	return client;
}

export function useApi(): ApiClient {
	if (!client) throw new Error('useApi() before provideApiClient()');
	return client;
}

export function usePlatform(): PlatformAPI {
	if (!platformApi) throw new Error('usePlatform() before provideApiClient()');
	return platformApi;
}
