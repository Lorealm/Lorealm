import type { Snowflake } from './core.ts';

export interface CharacterSummary {
	id: Snowflake;
	user_id: Snowflake;
	name: string;
	display_name?: string;
	tags: string[];
	avatar_uri?: string;
	card_version: string;
	updated_at: string;
}

export interface Character {
	id: Snowflake;
	user_id: Snowflake;
	current_version_id: Snowflake;
	metadata: CharacterMetadata;
	definition: CharacterDefinition;
	/** In order; the first is the main lorebook, used by single-lorebook exports. */
	lorebook_ids: Snowflake[];
	/** Available in this character's chats even when switched off globally. */
	tool_ids: Snowflake[];
	widget_ids: Snowflake[];
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
	template_injection: string;
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

/**
 * A lorebook created from a card's embedded `character_book`.
 */
export interface ImportedLorebook {
	id: Snowflake;
	name: string;
	/** Entries that converted cleanly. */
	entry_count: number;
	/** Entries dropped because their content could not be converted. */
	skipped: number;
}

export type ImportResult =
	| { status: 'ok'; id: Snowflake; lorebook?: ImportedLorebook }
	| { status: 'failed'; reason: string };

export interface ImportCharResponse {
	results: ImportResult[];
}

/**
 * A single edit applied against the current editing checkpoint.
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
	| { op: 'set_alt_names'; alt_names: string[] }
	| { op: 'set_widgets'; widget_ids: Snowflake[] }
	| { op: 'set_lorebooks'; lorebook_ids: Snowflake[] }
	| { op: 'set_tools'; tool_ids: Snowflake[] };

export type AttachmentKind = 'lorebook' | 'tool' | 'widget';

export interface DuplicateAttachmentResponse {
	/** The character, now pointing at the copy. */
	character: Character;
	new_id: Snowflake;
}

export interface EditDefinition {
	name?: string | null;
	description?: string | null;
	personality?: string | null;
	scenario?: string | null;
	system_prompt?: string | null;
	post_history_instructions?: string | null;
	template_injection?: string | null;
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

/**
 * Request body for creating a new character.
 */
export interface CreateCharacterRequest {
	name?: string;
}

export interface AvatarUpdateResponse {
	avatar_uri: string;
}
