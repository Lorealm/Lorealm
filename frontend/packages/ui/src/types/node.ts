// TypeScript type definitions converted from the Rust `mod.rs` source.
// These mirror the serde JSON wire format (snake_case variants, tag fields, etc.).
// No implementations are included — types only.

import type { Snowflake } from "./core.ts";

// ---------------------------------------------------------------------------
// External / cross-module types
// ---------------------------------------------------------------------------

/**
 * Role hint for the LLM. Referenced here as `super::PromptRole` but defined
 * in the parent module, so this is inferred from the doc comment
 * ("system, user, assistant"). Adjust if the actual enum differs.
 */
export type PromptRole = 'system' | 'user' | 'assistant';

/**
 * Arbitrary JSON value (`serde_json::Value`).
 */
export type JsonValue =
	| null
	| boolean
	| number
	| string
	| JsonValue[]
	| { [key: string]: JsonValue };

// ---------------------------------------------------------------------------
// Core template types
// ---------------------------------------------------------------------------

/**
 * The mode for a certain group node. Changes the behavior when updating.
 */
export type GroupMode =
	/** Makes the group act like a normal container. */
	| 'collection'
	/** Acts as a "toggle-group": only one item within the group may be selected. */
	| 'invariant';

/**
 * Defines what kind of node this is and its type-specific data.
 * Internally tagged on the `type` field (discriminated union).
 */
export type NodeContent = NodeContentText | NodeContentSource | NodeContentGroup;

/** User-editable text prompt. */
export interface NodeContentText {
	type: 'text';
	content: string;
	/** Role hint for the LLM (system, user, assistant). */
	role: PromptRole;
}

/** A node provided by a source (chat history, world info, extensions, etc.). */
export interface NodeContentSource {
	type: 'source';
	/** Which source provides this node (e.g., "chat_history", "world_info"). */
	source_id: string;
	/** Which specific node from that source (sources can offer multiple nodes). */
	node_id: string;
	/** Source-specific settings. */
	settings: Record<string, JsonValue>;
}

/** A grouping container for organization. */
export interface NodeContentGroup {
	type: 'group';
	children: ContextNode[];
	/** Optional text to prepend before group contents (null when absent). */
	prefix: string | null;
	/** Optional text to append after group contents (null when absent). */
	suffix: string | null;
	/** Whether prefix/suffix should be included even if all children are disabled. */
	always_include_wrappers: boolean;
	/** Which mode to use for this group. */
	mode: GroupMode;
}

/** A single node in the context template. */
export interface ContextNode {
	id: Snowflake;
	title: string;
	description: string;
	enabled: boolean;
	content: NodeContent;
}

/**
 * Path to a node within the template (list of indices at each level).
 * Rust: `Vec<usize>`.
 */
export type NodePath = number[];

// ---------------------------------------------------------------------------
// Source & setting definitions
// ---------------------------------------------------------------------------

/** Describes a setting that a source node accepts. */
export interface SettingDefinition {
	key: string;
	label: string;
	description: string;
	setting_type: SettingType;
	default_value: JsonValue;
	required: boolean;
}

/**
 * Internally tagged on the `type` field (discriminated union).
 * Variant names are snake_case (e.g. `TokenCount` -> "token_count").
 */
export type SettingType =
	| SettingTypeString
	| SettingTypeInteger
	| SettingTypeFloat
	| SettingTypeBoolean
	| SettingTypeSelect
	| SettingTypeTokenCount;

export interface SettingTypeString {
	type: 'string';
	/** Rust `Option<usize>`. */
	max_length: number | null;
}

export interface SettingTypeInteger {
	type: 'integer';
	/** Rust `Option<i64>`. */
	min: number | null;
	/** Rust `Option<i64>`. */
	max: number | null;
}

export interface SettingTypeFloat {
	type: 'float';
	/** Rust `Option<f64>`. */
	min: number | null;
	/** Rust `Option<f64>`. */
	max: number | null;
}

export interface SettingTypeBoolean {
	type: 'boolean';
}

export interface SettingTypeSelect {
	type: 'select';
	options: SelectOption[];
}

export interface SettingTypeTokenCount {
	type: 'token_count';
}

export interface SelectOption {
	value: string;
	label: string;
}

/** Describes a single node that a source can provide. */
export interface SourceNodeDefinition {
	/** Unique ID for this node within the source. */
	node_id: string;
	title: string;
	description: string;
	/** Icon identifier (e.g., "message-square", "book", "brain"). Rust `Option<String>`. */
	icon: string | null;
	/** Settings this node accepts. */
	settings: SettingDefinition[];
}

/** Describes a source that can provide context nodes. */
export interface SourceDefinition {
	/** Unique identifier for this source. */
	source_id: string;
	name: string;
	description: string;
	/** Whether this is a built-in source or from an extension. */
	source_type: SourceType;
	/** All nodes this source can provide. */
	nodes: SourceNodeDefinition[];
}

/**
 * SourceType uses serde's DEFAULT (external) tagging — it has no `#[serde(tag)]`.
 * That makes the JSON shape different from the internally-tagged enums above:
 *   - Unit variant `BuiltIn`  ->  the bare string "built_in"
 *   - Struct variant `Extension { extension_id }` -> { "extension": { "extension_id": "..." } }
 */
export type SourceType = 'built_in' | { extension: { extension_id: string } };
