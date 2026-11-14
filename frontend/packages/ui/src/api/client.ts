import type { PlatformAPI } from '#lib/core/api.js';
import type { LoginRequest, Session } from '#lib/types/auth.js';
import type {
	AttachmentKind,
	DuplicateAttachmentResponse,
	ExportFormat,
	ExportProfileId,
	ExportProfileInfo,
	ExportWarning,
	ImportedLorebook,
	ImportOptions,
	AvatarUpdateResponse,
	Character,
	CharacterEdit,
	CharacterSummary,
	CharacterVersionInfo,
	CreateCharacterRequest,
	ImportCharResponse
} from '#lib/types/character.js';
import type { ConfigKey, ServerConfig } from '#lib/types/config.js';
import type {
	ConnectionProfile,
	CreateSecretRequest,
	DeleteSecretResponse,
	ProfileRequest,
	RenameSecretRequest,
	SecretMetadata,
	SetActiveProfileRequest
} from '#lib/types/connection.js';
import type {
	ConversationMetadata,
	ChatEdit,
	ChatStreamEvent,
	ConversationMessage,
	FullConversation,
	StartChatRequest,
	TurnRequest
} from '#lib/types/conversation.js';
import type { JsonValue, Snowflake } from '#lib/types/core.js';
import type { ProviderModel } from '#lib/types/schema.js';
import type {
	Lorebook,
	LorebookMetadata,
	CreateLorebookRequest,
	LorebookEntry,
	UpdateEntryRequest,
	UpdateLorebookRequest,
	SetActiveLorebooksRequest
} from '#lib/types/lorebook.js';
import type { SourceDefinition } from '#lib/types/node.js';
import type {
	InstallReport,
	InstalledPlugin,
	Marketplace,
	PluginCallSlot,
	PluginEvent,
	PluginLog,
	PluginSettings
} from '#lib/types/plugins.js';
import type {
	Blueprint,
	BlueprintSummary,
	BuildForm,
	BundlePreview,
	CreateBlueprintRequest,
	ImportedBundle,
	InputSpec,
	OutlineResult,
	UpdateBlueprintRequest
} from '#lib/types/blueprint.js';
import type { CodeModule, CreateModuleRequest, UpdateModuleRequest } from '#lib/types/modules.js';
import type {
	WeaverCheckResponse,
	WeaverDialect,
	WeaverProfile,
	WeaverVocabulary
} from '#lib/types/weaver.js';
import type {
	UserPersona,
	UpdatePersona,
	PersonaOverride,
	SetPersonaOverride
} from '#lib/types/persona.js';
import type {
	ActionSpec,
	RealmResourceRef,
	NextTurn,
	Realm,
	RealmCast,
	RealmSummary,
	RealmVersionInfo,
	StartRealmChatRequest,
	StateSlice
} from '#lib/types/realm.js';
import type { ReconstructedState } from '#lib/types/state.js';
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
} from '#lib/types/template.js';
import type {
	BatchTokenizeRequest,
	BatchTokenizeResponse,
	NetworkCreate,
	ShareRequest,
	Tokenizer,
	TokenizeRequest,
	TokenizerShare
} from '#lib/types/tokenizer.js';
import type {
	CommandRun,
	CreateUserTool,
	RunCommandRequest,
	ToolEnabledRequest,
	UpdateUserTool,
	UserTool
} from '#lib/types/tools.js';
import type {
	CreateUserWidgetRequest,
	UpdateUserWidgetRequest,
	UserWidget
} from '#lib/types/widgets.js';
import type {
	AdminUpdateUserRequest,
	AdminUser,
	ChangePasswordRequest,
	CreatedInvite,
	CreateInviteRequest,
	CreateUserRequest,
	Invite,
	RegisterRequest,
	UpdateProfileRequest,
	User,
	UserSession
} from '#lib/types/user.js';
import type {
	CreateRoleRequest,
	MyPermissions,
	Role,
	UpdateRoleRequest
} from '#lib/types/permissions.js';
import type { TransferFile, TransferImportResponse, TransferKind } from '#lib/types/transfer.js';
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

function importQuery(options: ImportOptions): string {
	return options.reuseLorebooks ? '?reuse_lorebooks=true' : '';
}

export function createApiClient(platform: PlatformAPI) {
	const { http } = platform;

	return {
		auth: {
			login: (login: LoginRequest) => http.post<Session>('/api/v1/login', login)
		},
		users: {
			me: () => http.get<User>('/api/v1/users/me'),
			permissions: () => http.get<MyPermissions>('/api/v1/users/me/permissions'),
			get: (id: Snowflake) => http.get<User>(`/api/v1/users/${id}`),
			updateProfile: (update: UpdateProfileRequest) => http.patch<User>('/api/v1/users/me', update),
			changePassword: (request: ChangePasswordRequest) =>
				http.put('/api/v1/users/me/password', request),
			setAvatar: (avatar: Blob) =>
				http.putBlob<AvatarUpdateResponse>('/api/v1/users/me/avatar', avatar),
			sessions: {
				list: () => http.get<Array<UserSession>>('/api/v1/users/me/sessions'),
				revoke: (id: Snowflake) => http.delete(`/api/v1/users/me/sessions/${id}`),
				/** Sign out every device except this one. */
				revokeOthers: () => http.delete('/api/v1/users/me/sessions')
			},
			register: (request: RegisterRequest) => http.post<void>('/api/v1/users/register', request),
			/** Managing other people's accounts. Needs the "Manage Users" permission. */
			admin: {
				list: () => http.get<Array<AdminUser>>('/api/v1/users'),
				create: (request: CreateUserRequest) => http.post<AdminUser>('/api/v1/users', request),
				update: (id: Snowflake, update: AdminUpdateUserRequest) =>
					http.patch<AdminUser>(`/api/v1/users/${id}`, update),
				delete: (id: Snowflake) => http.delete<void>(`/api/v1/users/${id}`),
				/** Sign the user out on every device. */
				signOut: (id: Snowflake) => http.delete<void>(`/api/v1/users/${id}/sessions`)
			},
			invites: {
				list: () => http.get<Array<Invite>>('/api/v1/users/invites'),
				create: (request: CreateInviteRequest) =>
					http.post<CreatedInvite>('/api/v1/users/invites', request),
				delete: (id: Snowflake) => http.delete<void>(`/api/v1/users/invites/${id}`)
			}
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
			/** `profile` names the host; omitted, the checker uses every host's union. */
			weaverCheck: (source: string, dialect: WeaverDialect, profile?: WeaverProfile) =>
				http.post<WeaverCheckResponse>('/api/v1/templates/weaver-check', {
					source,
					dialect,
					profile
				}),
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
			exportSillyTavern: (id: Snowflake) =>
				http.get<{ file: unknown; warnings: ExportWarning[] }>(
					`/api/v1/lorebooks/${id}/export/sillytavern`
				),
			importSillyTavern: (file: unknown, name: string) =>
				http.post<ImportedLorebook>(
					`/api/v1/lorebooks/import/sillytavern?name=${encodeURIComponent(name)}`,
					file
				),
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
			/** The export profiles to offer. */
			exportProfiles: () => http.get<ExportProfileInfo[]>('/api/v1/characters/export/profiles'),
			exportWarnings: (characterId: Snowflake, profile: ExportProfileId, format: ExportFormat) =>
				http.get<ExportWarning[]>(
					`/api/v1/characters/${characterId}/export/warnings?profile=${encodeURIComponent(profile)}&format=${format}`
				),
			/** The card file: a PNG with the card embedded, or the card's JSON. */
			export: (characterId: Snowflake, profile: ExportProfileId, format: ExportFormat) =>
				http.getBlob(
					`/api/v1/characters/${characterId}/export?profile=${encodeURIComponent(profile)}&format=${format}`
				),
			/** Copies an attachment and points the character at the copy. */
			duplicateAttachment: (characterId: Snowflake, kind: AttachmentKind, id: Snowflake) =>
				http.post<DuplicateAttachmentResponse>(
					`/api/v1/characters/${characterId}/attachments/duplicate`,
					{ kind, id }
				),
			/** Deletes the character and every chat that uses it. */
			delete: (characterId: Snowflake) => http.delete(`/api/v1/characters/${characterId}`),
			avatar: (characterId: Snowflake) => http.getBlob(`/api/v1/characters/${characterId}/image`),
			updateAvatar: (characterId: Snowflake, update: Blob) =>
				http.putBlob<AvatarUpdateResponse>(`/api/v1/characters/${characterId}/image`, update),
			import: (card: Blob, options: ImportOptions = {}) =>
				http.putBlob<ImportCharResponse>(`/api/v1/characters/import${importQuery(options)}`, card),
			/** Imports every card in one request. Results come back in the order given. */
			importBatch: (cards: Blob[], options: ImportOptions = {}) =>
				http.putBlob<ImportCharResponse>(
					`/api/v1/characters/importBatch${importQuery(options)}`,
					encodeCardBatch(cards)
				),
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
			setAuthorNote: (conversation_id: Snowflake, author_note: string) =>
				http.patch(`/api/v1/chat/${conversation_id}/author-note`, { author_note }),
			/** Start a playthrough of a realm. */
			startRealm: (request: StartRealmChatRequest) =>
				http.post<FullConversation>('/api/v1/chat/realm', request),
			/** The cast this conversation plays with, pinned when it started. */
			cast: (conversation_id: Snowflake) =>
				http.get<RealmCast>(`/api/v1/chat/${conversation_id}/cast`),
			nextTurn: (conversation_id: Snowflake) =>
				http.get<NextTurn>(`/api/v1/chat/${conversation_id}/turn/next`),
			/** Advance the conversation and stream what happens. */
			turn: (conversation_id: Snowflake, request: TurnRequest) =>
				http.stream<ChatStreamEvent>(`/api/v1/chat/${conversation_id}/turn`, request),
			cancel: (conversation_id: Snowflake) => http.delete(`/api/v1/chat/${conversation_id}/turn`),
			agents: {
				actions: (conversation_id: Snowflake, handle: string) =>
					http.get<Array<ActionSpec>>(`/api/v1/chat/${conversation_id}/agents/${handle}/actions`),
				/** What the agent's grants let it read: at `at` when given, else the active leaf. */
				state: (conversation_id: Snowflake, handle: string, at?: Snowflake) =>
					http.get<StateSlice>(
						`/api/v1/chat/${conversation_id}/agents/${handle}/state${at !== undefined ? `?at=${at}` : ''}`
					)
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
			/** Fails for the last persona. If it was active, another one takes over. */
			delete: (persona_id: Snowflake) => http.delete(`/api/v1/persona/${persona_id}`),
			setAvatar: (persona_id: Snowflake, avatar: Blob) =>
				http.putBlob<AvatarUpdateResponse>(`/api/v1/persona/${persona_id}/avatar`, avatar),
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
			commands: () => http.get<Array<UserTool>>('/api/v1/tools/commands'),
			run: (tool_id: Snowflake, request: RunCommandRequest) =>
				http.post<CommandRun>(`/api/v1/tools/${tool_id}/run`, request),
			get: (tool_id: Snowflake) => http.get<UserTool>(`/api/v1/tools/${tool_id}`),
			create: (create: CreateUserTool) => http.post<Snowflake>('/api/v1/tools', create),
			update: (tool_id: Snowflake, update: UpdateUserTool) =>
				http.put<UserTool>(`/api/v1/tools/${tool_id}`, update),
			toggle: (tool_id: Snowflake, enable: ToolEnabledRequest) =>
				http.patch(`/api/v1/tools/${tool_id}`, enable),
			delete: (tool_id: Snowflake) => http.delete(`/api/v1/tools/${tool_id}`)
		},
		plugins: {
			list: () => http.get<Array<InstalledPlugin>>('/api/v1/plugins'),
			/** What the server's plugin indexes list. */
			marketplace: () => http.get<Marketplace>('/api/v1/plugins/marketplace'),
			/** A listed plugin's README, as Markdown. */
			listedReadme: async (handle: string) =>
				(await http.getBlob(`/api/v1/plugins/marketplace/${handle}/readme`)).text(),
			/** Installs a listed plugin for everyone, at the newest version the server can run. */
			installListed: (handle: string) =>
				http.post<InstallReport>(`/api/v1/plugins/marketplace/${handle}/install`, {}),
			toggle: (handle: string, enabled: boolean) =>
				http.patch(`/api/v1/plugins/${handle}`, { enabled }),
			/** To the latest installed version when `version` is left out. */
			update: (handle: string, version?: string) =>
				http.post(`/api/v1/plugins/${handle}/update`, { version }),
			install: (location: string, dev = false) =>
				http.post<InstallReport>('/api/v1/plugins/install', { location, dev }),
			/** Rebuilds a plugin installed for development and reloads it. */
			reload: (handle: string) => http.post<InstallReport>(`/api/v1/plugins/${handle}/reload`),
			/** Installs a zipped pre-built plugin. */
			upload: (zip: Blob) => http.putBlob<InstallReport>('/api/v1/plugins/install/upload', zip),
			uninstall: (handle: string) => http.delete(`/api/v1/plugins/${handle}`),
			/** The plugin's README, as Markdown. */
			readme: async (handle: string) =>
				(await http.getBlob(`/api/v1/plugins/${handle}/readme`)).text(),
			logs: (handle: string) => http.get<Array<PluginLog>>(`/api/v1/plugins/${handle}/logs`),
			settings: (handle: string) => http.get<PluginSettings>(`/api/v1/plugins/${handle}/settings`),
			saveSettings: (handle: string, values: Record<string, unknown>) =>
				http.put(`/api/v1/plugins/${handle}/settings`, values),
			/** A slot entry's HTML. */
			frontendEntry: async (handle: string, entry: string) =>
				(
					await http.getBlob(`/api/v1/plugins/${handle}/frontend/${encodeURIComponent(entry)}`)
				).text(),
			/**
			 * A call from the plugin's frontend to its own backend.
			 *
			 * `slot` says
			 * where the call came from: which conversation and message it is
			 * showing, and whether the user is waiting for the answer.
			 */
			call: (handle: string, name: string, args: JsonValue, slot: PluginCallSlot) =>
				http.post<JsonValue>(`/api/v1/plugins/${handle}/call`, {
					name,
					args,
					conversation: slot.conversation,
					message: slot.message,
					blocking: slot.blocking
				}),
			/** What the user's plugins send their frontends, from now on. */
			events: () => http.stream<PluginEvent>('/api/v1/plugins/events')
		},
		userWidgets: {
			list: () => http.get<Array<UserWidget>>('/api/v1/widgets'),
			builtins: () => http.get<Array<UserWidget>>('/api/v1/widgets/builtin'),
			get: (widget_id: Snowflake) => http.get<UserWidget>(`/api/v1/widgets/${widget_id}`),
			create: (create: CreateUserWidgetRequest) => http.post<UserWidget>('/api/v1/widgets', create),
			update: (widget_id: Snowflake, update: UpdateUserWidgetRequest) =>
				http.put<UserWidget>(`/api/v1/widgets/${widget_id}`, update),
			toggle: (widget_id: Snowflake, enabled: boolean) =>
				http.patch(`/api/v1/widgets/${widget_id}`, { enabled }),
			delete: (widget_id: Snowflake) => http.delete(`/api/v1/widgets/${widget_id}`)
		},
		transfer: {
			export: (kind: TransferKind, id: Snowflake) =>
				http.get<TransferFile>(`/api/v1/transfer/export/${kind}/${id}`),
			import: (file: TransferFile) =>
				http.post<TransferImportResponse>('/api/v1/transfer/import', file)
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
		modules: {
			list: () => http.get<Array<CodeModule>>('/api/v1/modules'),
			get: (id: Snowflake) => http.get<CodeModule>(`/api/v1/modules/${id}`),
			create: (create: CreateModuleRequest) => http.post<CodeModule>('/api/v1/modules', create),
			update: (id: Snowflake, update: UpdateModuleRequest) =>
				http.patch<CodeModule>(`/api/v1/modules/${id}`, update),
			delete: (id: Snowflake) => http.delete(`/api/v1/modules/${id}`)
		},
		blueprints: {
			list: () => http.get<Array<BlueprintSummary>>('/api/v1/blueprints'),
			get: (id: Snowflake) => http.get<Blueprint>(`/api/v1/blueprints/${id}`),
			create: (create: CreateBlueprintRequest) =>
				http.post<Blueprint>('/api/v1/blueprints', create),
			update: (id: Snowflake, update: UpdateBlueprintRequest) =>
				http.patch<Blueprint>(`/api/v1/blueprints/${id}`, update),
			delete: (id: Snowflake) => http.delete(`/api/v1/blueprints/${id}`),
			inputs: (id: Snowflake) => http.get<Array<InputSpec>>(`/api/v1/blueprints/${id}/inputs`),
			/** The author's introduction and the fields, for building a realm. */
			form: (id: Snowflake) => http.get<BuildForm>(`/api/v1/blueprints/${id}/form`),
			/** Its structure, for opening it in the editor. */
			outline: (id: Snowflake) => http.get<OutlineResult>(`/api/v1/blueprints/${id}/outline`),
			/** Expand a blueprint into a playable realm. */
			expand: (id: Snowflake, inputs: Record<string, unknown>) =>
				http.post<Realm>(`/api/v1/blueprints/${id}/realms`, { inputs }),
			export: (id: Snowflake) => http.getBlob(`/api/v1/blueprints/${id}/export`),
			previewImport: (file: Blob) =>
				http.putBlob<BundlePreview>('/api/v1/blueprints/import/preview', file),
			import: (file: Blob) => http.putBlob<ImportedBundle>('/api/v1/blueprints/import', file)
		},
		realms: {
			list: () => http.get<Array<RealmSummary>>('/api/v1/realms'),
			get: (realm_id: Snowflake) => http.get<Realm>(`/api/v1/realms/${realm_id}`),
			delete: (realm_id: Snowflake) => http.delete(`/api/v1/realms/${realm_id}`),
			/** Every resource the cast references, inferred server-side. */
			resources: (realm_id: Snowflake) =>
				http.get<Array<RealmResourceRef>>(`/api/v1/realms/${realm_id}/resources`),
			versions: {
				list: (realm_id: Snowflake) =>
					http.get<Array<RealmVersionInfo>>(`/api/v1/realms/${realm_id}/versions`),
				get: (realm_id: Snowflake, version_id: Snowflake) =>
					http.get<Realm>(`/api/v1/realms/${realm_id}/versions/${version_id}`)
			}
		},
		roles: {
			list: () => http.get<Array<Role>>('/api/v1/roles'),
			create: (role: CreateRoleRequest) => http.post<Role>('/api/v1/roles', role),
			update: (id: Snowflake, update: UpdateRoleRequest) =>
				http.patch<Role>(`/api/v1/roles/${id}`, update),
			delete: (id: Snowflake) => http.delete<void>(`/api/v1/roles/${id}`),
			members: {
				list: (id: Snowflake) => http.get<Array<User>>(`/api/v1/roles/${id}/members`),
				add: (id: Snowflake, username: string) =>
					http.post<User>(`/api/v1/roles/${id}/members`, { username }),
				remove: (id: Snowflake, userId: Snowflake) =>
					http.delete<void>(`/api/v1/roles/${id}/members/${userId}`)
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
