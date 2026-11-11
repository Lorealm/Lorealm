import type { Snowflake } from './core.js';

export interface CodeModule {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	source: string;
	enabled: boolean;
	created_at: string;
	updated_at: string;
}

export interface CreateModuleRequest {
	name: string;
	description?: string;
	source?: string;
}

export interface UpdateModuleRequest {
	name?: string;
	description?: string;
	source?: string;
	enabled?: boolean;
}
