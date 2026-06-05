import type { Snowflake } from './core.ts';

export interface CharacterSummary {
	id: Snowflake;
	user_id: Snowflake;
	name: string;
	display_name?: string;
	tags: string[];
	avatar_uri?: string;
	card_version: string;
	lorebook_id?: Snowflake;
	updated_at: string;
}

export interface Character {
	id: Snowflake;
	user_id: Snowflake;
	current_version_id: Snowflake;
	metadata: CharacterMetadata;
	definition: CharacterDefinition;
	lorebook_id?: Snowflake;
	/** CCv3 round-trip preservation. Opaque JSON blob. */
	extensions: unknown;
}

export interface CharacterVersionInfo {
	id: Snowflake;
	version_num: number;
	card_version: string;
	created_at: string;
}

export interface CharacterMetadata {
	display_name?: string;
	alt_names: string[];
	tags: string[];
	creator: string;
	creator_notes: string;
	card_version: string;
	avatar_uri?: string;
	created_at: string;
	updated_at: string;
}

export interface CharacterDefinition {
	name: string;
	description: string;
	personality: string;
	scenario: string;
	system_prompt: string;
	post_history_instructions: string;
	example_messages: ExampleDialogue[];
	greetings: Greeting[];
}

export interface Greeting {
	id: Snowflake;
	order: number;
	text: string;
}

export interface ExampleDialogue {
	id: Snowflake;
	order: number;
	content: string;
}

export type ImportResult = { status: 'ok'; id: Snowflake } | { status: 'failed'; reason: string };

export interface ImportCharResponse {
	results: ImportResult[];
}

/**
 * A single edit applied against the current editing checkpoint.
 *
 * The endpoint accepts a CharacterEdit[]; Order is preserved and ops apply in sequence.
 */
export type CharacterEdit =
	| ({ op: 'definition' } & EditDefinition)
	| ({ op: 'metadata' } & EditMetadata)
	// greetings
	| { op: 'add_greeting'; text: string }
	| { op: 'update_greeting'; id: Snowflake; text: string }
	| { op: 'remove_greeting'; id: Snowflake }
	| { op: 'reorder_greetings'; ids: Snowflake[] }
	// example dialogues
	| { op: 'add_example'; content: string }
	| { op: 'update_example'; id: Snowflake; content: string }
	| { op: 'remove_example'; id: Snowflake }
	| { op: 'reorder_examples'; ids: Snowflake[] }
	// whole-list replace
	| { op: 'set_tags'; tags: string[] }
	| { op: 'set_alt_names'; alt_names: string[] };

export interface EditDefinition {
	name?: string | null;
	description?: string | null;
	personality?: string | null;
	scenario?: string | null;
	system_prompt?: string | null;
	post_history_instructions?: string | null;
}

export interface EditMetadata {
	creator?: string | null;
	creator_notes?: string | null;
	card_version?: string | null;

	/**
	 * This field is a bit special, as it maps to Rust's `Option<Option<String>>`.
	 * Its important to be aware of how the server handles it:
	 *   omitted => leave unchanged
	 *   null    => clear to none
	 *   "x"     => set to "x"
	 */
	display_name?: string | null;
}
