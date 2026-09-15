import type { ConversationMessage } from './conversation.ts';
import type { JsonValue, PromptRole, Snowflake } from './core.ts';
import type { CommandEffect, Param } from './tools.ts';
import type { WidgetInstance } from './widgets.ts';

export interface Realm {
	id: Snowflake;
	owner_id: Snowflake;
	current_version_id: Snowflake;
	definition: RealmDefinition;
	tags: string[];
}

export interface RealmDefinition {
	name: string;
	description: string;
	/** Array order is turn order. */
	agents: RealmAgent[];
	scenarios: Scenario[];
	/** How the realm picks who acts next. Null = round-robin over the whole cast. */
	director?: Director | null;
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
	| { match: 'all' }
	| { match: 'kind'; kind: AgentKind }
	| { match: 'handles'; handles: string[] };

/**
 * A named opening, like a character greeting.
 */
export interface Scenario {
	id: Snowflake;
	name: string;
	description: string;
	opening?: string | null;
}

export interface CreateScenarioRequest {
	name: string;
	description?: string;
	opening?: string | null;
}

export type RealmResourceKind =
	| 'character'
	| 'lorebook'
	| 'template'
	| 'tool'
	| 'persona'
	| 'connection_profile'
	| 'widget';

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

export interface RealmSummary {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	tags: string[];
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
	speech_channel?: string | null;
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

export interface RealmCast {
	realm_id: Snowflake;
	version_id: Snowflake | null;
	agents: RealmAgent[];
	director?: Director | null;
}

export type EventKind = 'spoke' | 'acted' | 'state_changed' | 'system';

export interface RealmEvent {
	id: Snowflake;
	conversation_id: Snowflake;
	message_id: Snowflake;
	seq: number;
	/** Handle of the cast member that produced it; null for engine events. */
	source: string | null;
	channel: string;
	kind: EventKind;
	content: string;
	payload: JsonValue;
}

export interface AgentObservation {
	id: Snowflake;
	conversation_id: Snowflake;
	message_id: Snowflake;
	agent_handle: string;
	event_id: Snowflake | null;
	role: PromptRole;
	content: string;
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

export interface CreateRealmRequest {
	name: string;
}

export interface UpdateRealmRequest {
	name?: string | null;
	description?: string | null;
	tags?: string[] | null;
	/** Omitted leaves the director alone; null resets it to the default rotation. */
	director?: Director | null;
}

export interface RealmConfig {
	/** How another agent's speech reads in an observer's feed. `{name}`, `{content}`. */
	speech_format?: string | null;
	/** An `acted` event's content. `{name}`, `{tool}`. */
	action_format?: string | null;
	/** The actor's private note after a tool runs. `{tool}`, `{output}`. */
	tool_result_format?: string | null;
	/** A `state_changed` event's content. `{scope}`, `{changes}`. */
	state_change_format?: string | null;
	/** One written variable within a state change. `{key}`, `{value}`. */
	state_write_format?: string | null;
	/** One deleted variable within a state change. `{key}`. */
	state_delete_format?: string | null;
}

export const REALM_CONFIG_DEFAULTS: Required<{ [K in keyof RealmConfig]: string }> = {
	speech_format: '{name}: {content}',
	action_format: '[{name} used {tool}]',
	tool_result_format: '[{tool} result: {output}]',
	state_change_format: '[state {scope} changed: {changes}]',
	state_write_format: '{key} = {value}',
	state_delete_format: '-{key}'
};

export interface CreateAgentRequest {
	name: string;
	handle: string;
	kind: AgentKind;

	// character: llm only, both-or-neither
	character_id?: Snowflake | null;
	char_version_id?: Snowflake | null;

	// llm or player
	template_id?: Snowflake | null;
	profile_id?: Snowflake | null;
	persona_id?: Snowflake | null;

	/** Seeded as `agents:<handle>.<key>` at instantiation. Keys are dotted paths. */
	properties?: Record<string, JsonValue>;
	grants?: AgentGrant[];
	tools?: ToolBinding[];
	speech_channel?: string | null;
	lorebooks?: Snowflake[];
	/** Static display widgets shown when this agent is the viewer. */
	widgets?: Snowflake[];
	script?: string | null;
	run_condition?: string | null;
}

export interface StartRealmChatRequest {
	realm_id: Snowflake;
	/**
	 * The scenario to open with. Omitted: the realm's only scenario, or the
	 * legacy emergent start when the realm defines none.
	 */
	scenario_id?: Snowflake | null;
}

export interface PlayerTurnRequest {
	content?: string | null;
	action?: PlayerAction | null;
}

export interface PlayerAction {
	tool_id: Snowflake;
	args: JsonValue;
}

export interface RunTurnRequest {
	target?: Snowflake | null;
}

export interface TurnResult {
	message: ConversationMessage;
	events: RealmEvent[];
}

export interface NextTurn {
	/** Null when every agent is gated off at the active leaf. */
	handle: string | null;
}

export type RealmStreamEvent =
	| { type: 'awaiting_player'; handle: string }
	| { type: 'turn_started'; handle: string; name: string }
	| { type: 'delta'; content: string }
	| { type: 'reasoning'; content: string }
	| { type: 'widget'; widget: WidgetInstance }
	| { type: 'effect'; effect: CommandEffect }
	| { type: 'action'; name: string; is_error: boolean }
	| { type: 'committed'; message: ConversationMessage; events: RealmEvent[] };
