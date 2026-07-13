import type { ConversationMessage } from './conversation.ts';
import type { JsonValue, PromptRole, Snowflake } from './core.ts';
import type { Param } from './tools.ts';
import type { WidgetInstance } from './widgets.ts';

/**
 * A realm is a versioned preset (same machinery as characters): a head
 * pointing at the current of its immutable version snapshots. The
 * definition — name, description, cast — is a document inside the snapshot.
 * It carries no runtime state; playthroughs live in conversations.
 */
export interface Realm {
	id: Snowflake;
	owner_id: Snowflake;
	current_version_id: Snowflake;
	definition: RealmDefinition;
	/** Synced separately, not versioned (like character tags). */
	tags: string[];
}

export interface RealmDefinition {
	name: string;
	description: string;
	/** The cast, sorted by (turn_order, id); round-robin runs over it. */
	agents: RealmAgent[];
}

export interface RealmVersionInfo {
	id: Snowflake;
	version_num: number;
	created_at: string;
}

export interface RealmSummary {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	tags: string[];
	updated_at: string;
}

export type AgentKind = 'llm' | 'player';

/** One cast member as authored in the realm definition. */
export interface RealmAgent {
	id: Snowflake;
	name: string;
	kind: AgentKind;

	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	template_id: Snowflake | null;
	profile_id: Snowflake | null;

	user_id: Snowflake | null;

	persona_id: Snowflake | null;
	turn_order: number;

	/** What the agent can know: scoped access to state namespaces. */
	grants: AgentGrant[];
	/** What the agent can do. Empty = deliberately speak-only. */
	tools: ToolBinding[];
}

export type AgentAccess = 'read' | 'read_write';

export interface AgentGrant {
	scope: string;
	access: AgentAccess;
}

export interface ToolBinding {
	tool_id: Snowflake;
	/** Scope of the Acted event when this tool runs; null = private
	 * (only the actor observes its own action). */
	event_scope?: string | null;
}

/**
 * The cast snapshot pinned when a conversation instantiated its realm.
 * Preset edits never reach an in-flight playthrough — turns read only this.
 */
export interface RealmCast {
	realm_id: Snowflake;
	/** Which realm version this playthrough came from (null on casts
	 * pinned before versioning existed). */
	version_id: Snowflake | null;
	agents: RealmAgent[];
}

export type EventKind = 'spoke' | 'acted' | 'state_changed' | 'system';

/** What happened on a turn, anchored to the message that committed it. */
export interface RealmEvent {
	id: Snowflake;
	conversation_id: Snowflake;
	message_id: Snowflake;
	seq: number;
	source: Snowflake | null;
	scope: string;
	kind: EventKind;
	content: string;
	payload: JsonValue;
}

/** One entry of an agent's private feed, keyed by its turn's message. */
export interface AgentObservation {
	id: Snowflake;
	conversation_id: Snowflake;
	message_id: Snowflake;
	agent_id: Snowflake;
	event_id: Snowflake | null;
	role: PromptRole;
	content: string;
}

/** A bound tool projected for display. */
export interface ActionSpec {
	tool_id: Snowflake;
	name: string;
	display_name: string | null;
	description: string;
	params: Param[];
	event_scope: string | null;
}

/** An agent's visible slice of the world state, keyed by namespace. */
export type StateSlice = Record<string, JsonValue>;

export interface CreateRealmRequest {
	name: string;
}

/** Partial preset edit; omitted fields keep their value. */
export interface UpdateRealmRequest {
	name?: string | null;
	description?: string | null;
	tags?: string[] | null;
}

export interface CreateAgentRequest {
	name: string;
	kind: AgentKind;

	// llm only — template/profile default from the caller's actives if omitted
	character_id?: Snowflake | null;
	char_version_id?: Snowflake | null;
	template_id?: Snowflake | null;
	profile_id?: Snowflake | null;

	persona_id?: Snowflake | null;

	grants?: AgentGrant[];
	/** The agent's action space. Empty = speak-only. */
	tools?: ToolBinding[];
}

export interface StartRealmChatRequest {
	realm_id: Snowflake;
}

export interface PlayerTurnRequest {
	content?: string | null;
	action?: PlayerAction | null;
}

export interface PlayerAction {
	tool_id: Snowflake;
	/** Arguments matching the tool's params, keyed by name. */
	args: JsonValue;
}

/** Regenerate a specific turn: the new message becomes a sibling of
 * `target` (same agent, alternate world-line). Omitted = advance the path. */
export interface RunTurnRequest {
	target?: Snowflake | null;
}

/** A committed turn: the message it anchored to and what fanned out of it. */
export interface TurnResult {
	message: ConversationMessage;
	events: RealmEvent[];
}

/** What a client watching a live turn receives over SSE. */
export type RealmStreamEvent =
	/** The policy picked a player; nothing will stream. Prompt that user. */
	| { type: 'awaiting_player'; agent_id: Snowflake }
	| { type: 'turn_started'; agent_id: Snowflake; name: string }
	| { type: 'delta'; content: string }
	| { type: 'reasoning'; content: string }
	| { type: 'widget'; widget: WidgetInstance }
	/** A tool ran. Thin by design: inputs/results stay actor-private. */
	| { type: 'action'; name: string; is_error: boolean }
	/** Terminal: the turn committed as this message with these events. */
	| { type: 'committed'; message: ConversationMessage; events: RealmEvent[] };
