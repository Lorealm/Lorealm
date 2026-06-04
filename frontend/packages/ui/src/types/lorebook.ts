// Lorebook + entry types. Storage-facing types match the server's wire shape;
// EntryMeta / Slot / LorebookConfig mirror the parsed `.weaver` frontmatter and
// `lorebook.yaml`. Frontmatter round-tripping lives in weaver/weaverFile.ts.

import type { Snowflake } from './core.js';

// ── Storage-facing (wire) types ────────────────────────────────────────────

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
	/** ContextWeaver id from the frontmatter (`id:`). Human-readable, referenced
	 *  by triggers and document refs. May collide across entries — that is
	 *  breaking, surfaced later by a validator, not enforced here. */
	entry_id: string;
	/** The entire raw `.weaver` file: frontmatter + body. The full round-trip
	 *  (parse for the form, serialize on save) is the frontend's responsibility. */
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

// ── Requests ────────────────────────────────────────────────────────────────

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

/** The patch is the full updated `.weaver` source. The server re-validates it
 *  and rejects invalid frontmatter / weaver syntax, so callers must preserve
 *  unrelated content exactly. */
export interface UpdateEntryRequest {
	source: string;
}

// ── Slot ──────────────────────────────────────────────────────────────────

/**
 * Slot enum — unit variants serialize as snake_case strings; `AtDepth` is an
 * externally-tagged tuple variant: `{ at_depth: [n] }`.
 */
export type Slot =
	| 'prelude'
	| 'preamble'
	| 'backdrop'
	| 'setting'
	| 'foreground'
	| 'coda'
	| { at_depth: number }; // newtype variant: { at_depth: N }

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

// ── EntryMeta (parsed frontmatter) ──────────────────────────────────────────

/**
 * The parsed view of an entry's YAML frontmatter. Optional fields are
 * `undefined` when absent from the source (NOT defaulted) so the serializer can
 * tell "author left this out" from "author set it" — display defaults are
 * applied at the form layer, never written back unless the author edits.
 *
 * Unknown frontmatter keys are intentionally absent here: they live only in the
 * preserved YAML document and survive untouched across a round-trip.
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

// ── lorebook.yaml config ────────────────────────────────────────────────────

export type NamespaceAccess = 'readonly' | 'readwrite';

export interface NamespaceConfig {
	access: NamespaceAccess;
	description: string;
}

/**
 * Parsed `lorebook.yaml`. Unlike entries this is NOT round-tripped through the
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
