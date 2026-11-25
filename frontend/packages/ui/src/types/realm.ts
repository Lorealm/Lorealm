import type { JsonValue, Snowflake } from './core.ts';
import type { Param } from './tools.ts';

export interface Realm {
	id: Snowflake;
	owner_id: Snowflake;
	current_version_id: Snowflake;
	definition: RealmDefinition;
	tags: string[];
	origin: RealmOrigin | null;
}

export interface RealmDefinition {
	name: string;
	description: string;
	/** Array order is turn order. */
	agents: RealmAgent[];
	scenarios: Scenario[];
	/** Authored labels. They live here so they travel with an export. */
	tags: string[];
	/** How the realm picks who acts next. Null = round-robin over the whole cast. */
	director?: Director | null;
	/** How events read in an agent's feed. */
	config?: RealmConfig | null;
	/** Modules the realm imports: active in its turns even while disabled elsewhere. */
	modules?: Snowflake[];
}

/**
 * How a realm decides who acts next.
 */
export type Director =
	| { kind: 'round_robin'; over: AgentFilter }
	| { kind: 'random'; over: AgentFilter }
	/** A weaver expression naming a handle, or an array of handles to try in order. */
	| { kind: 'script'; source: string };

export type AgentFilter =
	{ match: 'all' } | { match: 'kind'; kind: AgentKind } | { match: 'handles'; handles: string[] };

/**
 * A named opening, like a character greeting.
 */
export interface Scenario {
	slug: string;
	name: string;
	description: string;
	opening?: string | null;
}

export type RealmResourceKind =
	| 'character'
	| 'lorebook'
	| 'template'
	| 'tool'
	| 'persona'
	| 'connection_profile'
	| 'widget'
	| 'module';

/** One resource the realm references. Derived server-side from the cast. */
export interface RealmResourceRef {
	kind: RealmResourceKind;
	resource_id: Snowflake;
}

export interface RealmVersionInfo {
	id: Snowflake;
	version_num: number;
	created_at: string;
}

/** Where a realm came from: the blueprint, and the inputs it was built with. */
export interface RealmOrigin {
	blueprint_id: Snowflake;
	inputs: Record<string, JsonValue>;
}

export interface RealmSummary {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	tags: string[];
	origin: RealmOrigin | null;
	updated_at: string;
}

export type AgentKind = 'llm' | 'player' | 'script';

export interface RealmAgent {
	/**
	 * The agent's identity, everywhere: its `agents:<handle>` state subtree,
	 * the source of its events, and the key of its observation feed.
	 */
	handle: string;
	name: string;
	kind: AgentKind;

	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	template_id: Snowflake | null;
	profile_id: Snowflake | null;

	user_id: Snowflake | null;

	persona_id: Snowflake | null;

	/** Seeded as `agents:<handle>.<key>` at instantiation. Keys are dotted paths. */
	properties: Record<string, JsonValue>;
	grants: AgentGrant[];
	tools: ToolBinding[];
	lorebooks: Snowflake[];
	/** Static display widgets shown when this agent is the viewer. */
	widgets: Snowflake[];
	script?: string | null;
	run_condition?: string | null;
}

export type AgentAccess = 'read' | 'read_write';

export interface AgentGrant {
	channel: string;
	access: AgentAccess;
}

export interface ToolBinding {
	tool_id: Snowflake;
	event_channel?: string | null;
}

/** The cast a conversation plays with. Every conversation has one. */
export interface RealmCast {
	version_id: Snowflake | null;
	agents: RealmAgent[];
	director?: Director | null;
}

export interface DisplayCast extends RealmCast {
	character_widgets: Snowflake[];
}

export type EventKind = 'spoke' | 'acted' | 'system';

/** Something a turn did besides speak, kept in its message's `events` metadata. */
export interface TurnEvent {
	channel: string;
	kind: EventKind;
	content: string;
	observers?: string[];
}

export interface ActionSpec {
	tool_id: Snowflake;
	name: string;
	display_name: string | null;
	description: string;
	params: Param[];
	event_channel: string | null;
}

export type StateSlice = Record<string, JsonValue>;

export interface RealmConfig {
	/** How another agent's speech reads in an observer's feed. `{name}`, `{content}`. */
	speech_format?: string | null;
	/** An `acted` event's content. `{name}`, `{tool}`. */
	action_format?: string | null;
	/** The actor's private note after a tool runs. `{tool}`, `{output}`. */
	tool_result_format?: string | null;
}

export const REALM_CONFIG_DEFAULTS: Required<{ [K in keyof RealmConfig]: string }> = {
	speech_format: '{name}: {content}',
	action_format: '[{name} used {tool}]',
	tool_result_format: '[{tool} result: {output}]'
};

export interface StartRealmChatRequest {
	realm_id: Snowflake;
	scenario?: string | null;
}

/** One of the player's bound tools, or Weaver a widget sends. */
export type PlayerAction = { tool_id: Snowflake; args: JsonValue } | { run: string };

export interface NextTurn {
	/** Null when nobody in the cast can act at the active leaf. */
	handle: string | null;
}
