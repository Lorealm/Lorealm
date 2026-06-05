import type { Snowflake } from './core.js';

export interface Lorebook {
	id: Snowflake;
	owner_id: Snowflake;
	current_version_id: Snowflake;
	name: string;
	description: string;
	config_yaml: string;
	schema_version: number;
	created_at: string;
	updated_at: string;
	entries: LorebookEntry[];
}

export interface LorebookEntry {
	id: Snowflake;
	lorebook_id: Snowflake;
	entry_id: string;
	source: string;
}

export interface LorebookVersionInfo {
	id: Snowflake;
	version_num: number;
	book_version: string;
	created_at: string;
}

export interface LorebookMetadata {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	entry_count: string;
}

export interface CreateLorebookRequest {
	name: string;
	description?: string;
}

export interface SetActiveLorebooksRequest {
  ids: Snowflake[]
}

export interface UpdateLorebookRequest {
	name?: string;
	description?: string;
	config?: LorebookConfig;
}

export interface UpdateEntryRequest {
	source: string;
}

export type Slot =
	| 'prelude'
	| 'preamble'
	| 'backdrop'
	| 'setting'
	| 'foreground'
	| 'coda'
	| { at_depth: number };

/** Flattened enum as array for easier iterator access */
export const SLOT_KINDS = [
	'prelude',
	'preamble',
	'backdrop',
	'setting',
	'foreground',
	'coda',
	'at_depth'
] as const;

export function isAtDepth(slot: Slot): slot is { at_depth: number } {
	return typeof slot === 'object' && slot !== null && 'at_depth' in slot;
}

/**
 * The parsed view of an entry's YAML frontmatter. Optional fields are
 * `undefined` when absent from the source, not defaulted.
 *
 * Unknown frontmatter keys are intentionally absent here: they live only in the
 * preserved YAML document and survive untouched across a round-trip. This behavior
 * is a temporary band-aid may need to be changed or revised in the future.
 */
export interface EntryMeta {
	id: string;
	name?: string;

	// Activation
	keywords?: string[];
	regex?: string[];
	condition?: string;
	scan_depth?: number;
	constant?: boolean;

	// Ordering & placement
	priority?: number;
	slot?: Slot;
	fallback?: Slot[];
	insertion_order?: number;

	// Behavior
	enabled?: boolean;
	sticky_turns?: number;
	cooldown?: number;
	group?: string;
	tags?: string[];
}

export type NamespaceAccess = 'readonly' | 'readwrite';

export interface NamespaceConfig {
	access: NamespaceAccess;
	description: string;
}

/**
 * Parsed lorebook frontmatter. Unlike entries this is NOT round-tripped through the
 * frontend: it is sent back whole as `UpdateLorebookRequest.config` and the
 * server re-serializes, so the `& Record<string, unknown>` extension keys
 * survive as long as the full object is passed through unmodified.
 */
export type LorebookConfig = {
	name: string;
	description: string;
	namespaces: Record<string, NamespaceConfig>;
	default_scan_depth: number;
	default_priority: number;
	default_slot: Slot;
	token_budget: number | null;
	group_budgets: Record<string, number>;
	case_sensitive_keywords: boolean;
} & Record<string, unknown>;
