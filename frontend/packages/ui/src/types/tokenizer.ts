import type { Snowflake } from './core.ts';

export interface Tokenizer {
	id: Snowflake;
	name: string;
	blob_uri: string;
	owner_id: Snowflake;
	is_public: boolean;
}

export interface TokenizerShare {
	tok_id: Snowflake;
	user_id: Snowflake;
}

export interface ImportQuery {
	name: string;
	is_public: boolean;
}

export interface NetworkCreate {
	source: string;
	name?: string;
	is_public: boolean;
}

export interface ShareRequest {
	user_id: Snowflake;
}

export interface TokenizeRequest {
	content: string;
}

export interface BatchTokenizeRequest {
	batch: Map<string, string>;
}

export interface BatchTokenizeResponse {
	tokenizer_id: Snowflake;
	counts: Map<string, number>;
	total_tokens: number;
}