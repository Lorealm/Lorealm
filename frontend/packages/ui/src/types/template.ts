import type { Snowflake } from './core.ts';
import type { ContextNode, NodeContent, NodePath } from './node.ts';

export interface CreateTemplateRequest {
	name?: string;
	description?: string;
	default: boolean;
}

export interface UpdateTemplateRequest {
	name?: string;
	description?: string;
}

export interface CreateNodeRequest {
	title: string;
	description: string;
	content: NodeContent;
	parent_path?: NodePath;
	index?: number;
}

export interface NodeResponse {
	node: ContextNode;
	path: NodePath;
}

export interface UpdateNodeRequest {
	title?: string;
	description?: string;
	enabled?: boolean;
	content?: NodeContent;
}

export interface MoveNodeRequest {
	new_parent_path?: NodePath;
	new_index: number;
}

export interface MoveNodeResponse {
	new_path: NodePath;
}

export interface ToggleNodeResponse {
	delta: Record<Snowflake, boolean>;
}

/** The full template definition. */
export interface ContextTemplate {
	id: Snowflake;
	name: string;
	description: string;
	/** The user who owns this template. */
	owner_id: Snowflake;
	/** Root-level nodes (can include groups which have their own children). */
	nodes: ContextNode[];
	/** Template format version for migrations (Rust `u32`). */
	version: number;
	/** When the template was created (ISO 8601 / RFC 3339 string string). */
	created_at: string;
	/** When the template was last modified (ISO 8601 / RFC 3339 string string). */
	updated_at: string;
}

export interface TemplateMetadata {
	id: Snowflake;
	owner_id: Snowflake;
	name: string;
	description: string;
	node_count: number;
	version: number;
	created_at: string;
	updated_at: string;
}
