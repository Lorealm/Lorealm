import type { Snowflake } from './core.ts';

export type ParamType = 'string' | 'number' | 'bool' | 'object' | { array: ParamType };

export interface Param {
	name: string;
	ty: ParamType;
	description: string | null;
	required: boolean;
}

export interface UserTool {
	user_id: Snowflake;
	name: string;
	id: Snowflake;
	display_name: string | null;
	description: string;
	params: Param[];
	body: string;
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
}

export interface UpdateUserTool {
	name?: string;
	display_name?: string | null;
	description?: string;
	params?: Param[];
	body?: string;
}

export interface ToolEnabledRequest {
	enabled: boolean;
}
