import type { JsonValue, Snowflake } from './core.ts';

export interface SecretMetadata {
	id: Snowflake;
	user_id: Snowflake;
	provider: string;
	label: string;
	key_hint: string;
	created_at: string;
	updated_at: string;
}

export interface CreateSecretRequest {
	provider: string;
	label: string;
	key: string;
}

export interface RenameSecretRequest {
	label: string;
}

export interface DeleteSecretResponse {
	/** Profiles that were detached (their `secret_id` is now null). */
	detached_profiles: Snowflake[];
}

export interface ProfileRequest {
	secret_id?: Snowflake | null;
	name: string;
	provider: string;
	token_budget: number;
	model: string;
	common_params?: JsonValue;
	provider_params?: JsonValue;
}

export interface ConnectionProfile {
	id: Snowflake;
	user_id: Snowflake;
	secret_id: Snowflake | null;
	name: string;
	provider: string;
	model: string;
	token_budget: number;
	common_params: JsonValue;
	provider_params: JsonValue;
	created_at: string;
	updated_at: string;
}

export interface SetActiveProfileRequest {
	id?: Snowflake;
}
