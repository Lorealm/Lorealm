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
	/** Optional ownership of a lorebook. null = no embedded lorebook. */
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
	/** defaults to definition.name */
	display_name?: string;
	/** for search/sort */
	alt_names: string[];
	tags: string[];
	creator: string;
	creator_notes: string;
	/** the HUMAN-facing version string ("1.2.0") */
	card_version: string;
	avatar_uri?: string;
	created_at: string;
	updated_at: string;
}

export interface CharacterDefinition {
	/** in-chat name, {{char}} substitution target */
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

// Internally tagged on `status`, variants renamed to snake_case.
export type ImportResult = { status: 'ok'; id: Snowflake } | { status: 'failed'; reason: string };

export interface ImportCharResponse {
	results: ImportResult[];
}

/**
 * A single edit applied against the current editing checkpoint.
 *
 * The endpoint accepts a CharacterEdit[]; all ops in one request land in the
 * same debounced version. Order is preserved and ops apply in sequence.
 */
export type CharacterEdit =
	// scalar field groups (merge-patch). These are newtype variants in an
	// internally-tagged serde enum, so the inner struct's fields are flattened
	// alongside `op` rather than nested.
	| ({ op: 'definition' } & EditDefinition)
	| ({ op: 'metadata' } & EditMetadata)
	// greetings (item-level, identity via Snowflake)
	| { op: 'add_greeting'; text: string }
	| { op: 'update_greeting'; id: Snowflake; text: string }
	| { op: 'remove_greeting'; id: Snowflake }
	| { op: 'reorder_greetings'; ids: Snowflake[] }
	// example dialogues (item-level, identity via Snowflake)
	| { op: 'add_example'; content: string }
	| { op: 'update_example'; id: Snowflake; content: string }
	| { op: 'remove_example'; id: Snowflake }
	| { op: 'reorder_examples'; ids: Snowflake[] }
	// whole-list replace (no per-item identity needed)
	| { op: 'set_tags'; tags: string[] }
	| { op: 'set_alt_names'; alt_names: string[] };

/**
 * Partial patch over CharacterDefinition's scalar fields.
 * Omitted (or null) = leave the field unchanged. There is no "clear" state for
 * these fields — null and omission mean the same thing.
 */
export interface EditDefinition {
	name?: string | null;
	description?: string | null;
	personality?: string | null;
	scenario?: string | null;
	system_prompt?: string | null;
	post_history_instructions?: string | null;
}

/**
 * Partial patch over CharacterMetadata's editable scalar fields.
 *
 * avatar_uri is intentionally absent — it's owned by the image endpoint.
 * created_at / updated_at are server-managed.
 */
export interface EditMetadata {
	creator?: string | null;
	creator_notes?: string | null;
	card_version?: string | null;

	/**
	 * Three-state (from Rust's Option<Option<String>>):
	 *   omitted => leave unchanged
	 *   null    => clear to none
	 *   "x"     => set to "x"
	 *
	 * Unlike the fields above, here null is meaningfully different from omitting.
	 */
	display_name?: string | null;
}
