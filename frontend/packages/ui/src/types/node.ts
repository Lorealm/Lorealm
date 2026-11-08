import type { JsonValue, PromptRole, Snowflake } from './core.ts';

/**
 * The mode for a certain group node. Changes the behavior when updating.
 */
export type GroupMode =
	/** Makes the group act like a normal container. */
	| 'collection'
	/** Acts as a "toggle-group": only one item within the group may be selected. */
	| 'invariant';

export type GroupWrapMode = 'no_mixed' | 'majority' | 'all' | { only: PromptRole };

export type NodeContent = NodeContentText | NodeContentSource | NodeContentGroup;

/** User-editable text prompt. */
export interface NodeContentText {
	type: 'text';
	content: string;
	role: PromptRole;
}

/** A node provided by a source */
export interface NodeContentSource {
	type: 'source';
	source_id: string;
	node_id: string;
	/** Source-specific settings. */
	settings: Record<string, JsonValue>;
}

/** A grouping container for organization. */
export interface NodeContentGroup {
	type: 'group';
	children: ContextNode[];
	prefix: string | null;
	suffix: string | null;
	/** Whether prefix/suffix should be included even if all children are disabled. */
	always_include_wrappers: boolean;
	mode: GroupMode;
	wrap_mode: GroupWrapMode;
}

/** A single node in the context template. */
export interface ContextNode {
	id: Snowflake;
	title: string;
	description: string;
	enabled: boolean;
	content: NodeContent;
}

export type NodePath = number[];

/** A setting for nodes */
export interface SettingDefinition {
	key: string;
	label: string;
	description: string;
	setting_type: SettingType;
	default_value: JsonValue;
	required: boolean;
}

export type SettingType =
	| SettingTypeString
	| SettingTypeText
	| SettingTypeInteger
	| SettingTypeFloat
	| SettingTypeBoolean
	| SettingTypeSelect
	| SettingTypeTokenCount;

export interface SettingTypeString {
	type: 'string';
	max_length: number | null;
}

export interface SettingTypeText {
	type: 'text';
}

export interface SettingTypeInteger {
	type: 'integer';
	min: number | null;
	max: number | null;
}

export interface SettingTypeFloat {
	type: 'float';
	min: number | null;
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

/** The definition of a source node */
export interface SourceNodeDefinition {
	node_id: string;
	title: string;
	description: string;
	/** Icon identifier. Currently unused. */
	icon: string | null;
	/** Node-specific settings. */
	settings: SettingDefinition[];
}

/** The description of a source. Basically a container of node definitions */
export interface SourceDefinition {
	source_id: string;
	name: string;
	description: string;
	/** Whether this is a built-in source or from an extension. */
	source_type: SourceType;
	nodes: SourceNodeDefinition[];
}

export type SourceType = 'built_in' | { extension: { extension_id: string } };
