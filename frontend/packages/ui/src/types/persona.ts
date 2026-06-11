import type { Snowflake } from './core.ts';

export interface UserPersona {
	id: Snowflake;
	user_id: Snowflake;
	name: string;
	display_name: string | null;
	pseudonym: string | null;
	avatar_uri: string | null;
	description: string | null;
	updated_at: string;
}

export interface UpdatePersona {
	name?: string;
	display_name?: string | null;
	pseudonym?: string | null;
	description?: string | null;
}
