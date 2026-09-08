import type { Snowflake } from './core.ts';

export interface User {
	id: Snowflake;
	username: string;
	display_name: string | null;
	avatar_uri: string | null;
	created_at: string;
}

export interface UpdateProfileRequest {
	display_name?: string | null;
}

export interface UserSession {
	id: Snowflake;
	ip_address: string | null;
	user_agent: string | null;
	created_at: string;
	expires_at: string;
	last_rotated_at: string;
	current: boolean;
}
