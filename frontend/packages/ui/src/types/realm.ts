import type { JsonValue, PromptRole, Snowflake } from './core.ts';
import type { Param } from './tools.ts';
import type { WidgetInstance } from './widgets.ts';

export type RealmStatus = 'active' | 'paused' | 'archived';

export interface Realm {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	tick: number;
	state: JsonValue;
	tokenizer_id: Snowflake;
	status: RealmStatus;
}

export interface RealmSummary {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	status: RealmStatus;
}

export type AgentKind = 'llm' | 'player';

export interface RealmAgent {
	id: Snowflake;
	realm_id: Snowflake;
	name: string;
	kind: AgentKind;

	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	template_id: Snowflake | null;
	profile_id: Snowflake | null;

	user_id: Snowflake | null;

	persona_id: Snowflake | null;
	turn_order: number;
	extensions: JsonValue;
	created_at: string;
	updated_at: string;
}

export type AgentAccess = 'read' | 'read_write';

export interface AgentGrant {
	agent_id: Snowflake;
	scope: string;
	access: AgentAccess;
}

export interface ToolBinding {
	tool_id: Snowflake;
	/** Scope of the Acted event when this tool runs; null = private
	 * (only the actor observes its own action). */
	event_scope?: string | null;
}

export type EventKind = 'spoke' | 'acted' | 'state_changed' | 'system';

export interface RealmEvent {
	id: Snowflake;
	realm_id: Snowflake;
	tick: number;
	seq: number;
	source: Snowflake | null;
	scope: string;
	kind: EventKind;
	content: string;
	payload: JsonValue;
}

export interface AgentObservation {
	id: Snowflake;
	agent_id: Snowflake;
	event_id: Snowflake | null;
	tick: number;
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
	tokenizer_id?: Snowflake | null;
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

	grants?: GrantRequest[];
	/** The agent's action space. Empty = speak-only. */
	tools?: ToolBinding[];
}

export interface GrantRequest {
	scope: string;
	access: AgentAccess;
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

export type TickResponse =
	| { status: 'committed'; tick: number; events: RealmEvent[] }
	| { status: 'awaiting_player'; agent_id: Snowflake };

/** What a client watching a live tick receives over SSE. */
export type RealmStreamEvent =
	/** The policy picked a player; nothing will stream. Prompt that user. */
	| { type: 'awaiting_player'; agent_id: Snowflake }
	| { type: 'turn_started'; agent_id: Snowflake; name: string }
	| { type: 'delta'; content: string }
	| { type: 'reasoning'; content: string }
	| { type: 'widget'; widget: WidgetInstance }
	/** A tool ran. Thin by design: inputs/results stay actor-private. */
	| { type: 'action'; name: string; is_error: boolean }
	/** Terminal: the tick committed with these events. */
	| { type: 'committed'; tick: number; events: RealmEvent[] };
