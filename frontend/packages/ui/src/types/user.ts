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

export interface ChangePasswordRequest {
	current_password: string;
	new_password: string;
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

export interface RegisterRequest {
	username: string;
	email?: string;
	password: string;
	/** Lets you register while registration is disabled. */
	invite_code?: string;
}

/** A user as seen by someone who manages accounts. */
export interface AdminUser extends User {
	email: string | null;
}

export interface CreateUserRequest {
	username: string;
	email?: string;
	password: string;
}

export interface AdminUpdateUserRequest {
	email?: string;
	display_name?: string | null;
	/** Signs the user out everywhere. */
	password?: string;
}

export interface Invite {
	id: Snowflake;
	/** Given to whoever registers with the code. */
	role_id: Snowflake | null;
	created_by: Snowflake | null;
	/** Null means unlimited. */
	max_uses: number | null;
	uses: number;
	expires_at: string | null;
	created_at: string;
}

export interface CreateInviteRequest {
	role_id?: Snowflake;
	max_uses?: number;
	expires_at?: string;
}

/** The code is only shown once, when the invite is created. */
export interface CreatedInvite extends Invite {
	code: string;
}
