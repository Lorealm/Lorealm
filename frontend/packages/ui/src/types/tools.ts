import type { Snowflake } from './core.ts';
import type { ConversationMessage } from './conversation.ts';

export type ParamType = 'string' | 'number' | 'bool' | 'object' | { array: ParamType };

export interface Param {
	name: string;
	ty: ParamType;
	description: string | null;
	required: boolean;
}

/** Where a row is callable from: by the model, by the user, or both. */
export type ToolSurface = 'tool' | 'command' | 'both';

export interface UserTool {
	user_id: Snowflake;
	name: string;
	id: Snowflake;
	display_name: string | null;
	description: string;
	params: Param[];
	body: string;
	surface: ToolSurface;
	enabled: boolean;
	created_at: string;
	updated_at: string;
}

export interface CreateUserTool {
	name: string;
	display_name?: string | null;
	description: string;
	params: Param[];
	body: string;
	surface?: ToolSurface;
}

export interface UpdateUserTool {
	name?: string;
	display_name?: string | null;
	description?: string;
	params?: Param[];
	body?: string;
	surface?: ToolSurface;
}

export interface ToolEnabledRequest {
	enabled: boolean;
}

/** Something a command body asked the client to do. */
export type CommandEffect =
	| {
			type: 'toast';
			content: string;
			variant: 'info' | 'success' | 'warning' | 'error';
	  }
	| {
			type: 'compose';
			content: string;
			mode: 'replace' | 'append';
	  };

export interface RunCommandRequest {
	conversation_id: Snowflake;
	named: Record<string, string>;
	rest: string;
}

export interface CommandRun {
	/** The body's typed return value; `null` when it returned nothing. */
	output: unknown;
	effects: CommandEffect[];
	/** The system message the run anchored its state writes to, if any. */
	message: ConversationMessage | null;
}
