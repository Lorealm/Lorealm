import type { JsonValue, PromptRole, Snowflake } from './core.ts';
import type { PlayerAction } from './realm.ts';
import type { CommandEffect } from './tools.ts';
import type { WidgetInstance } from './widgets.ts';

export interface Conversation {
	id: Snowflake;
	user_id: Snowflake;
	/** The character this chat was started with, if it was started with one. */
	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	/** The realm this playthrough was started from, if it was started from one. */
	realm_id: Snowflake | null;
	active_leaf_id: Snowflake | null;
	title: string | null;
	author_note: string;
	created_at: string;
	updated_at: string;
}

export interface ConversationMessage {
	id: Snowflake;
	conversation_id: Snowflake;
	parent: Snowflake | null;
	role: PromptRole;
	content: string;
	order: number;
	metadata: JsonValue;
	created_at: string;
	updated_at: string;
}

export interface StoredImage {
	/** `sha256:` URI of the stored image. */
	uri: string;
}

/** The `sha256:` URIs of the images a message carries. */
export function messageImages(metadata: unknown): string[] {
	if (!metadata || typeof metadata !== 'object') return [];
	const images = (metadata as Record<string, unknown>).images;
	return Array.isArray(images) ? images.filter((i): i is string => typeof i === 'string') : [];
}

export interface ConversationMetadata {
	id: Snowflake;
	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	realm_id: Snowflake | null;
	title: string | null;
	length: number;
	created_at: string;
	updated_at: string;
}

export interface StartChatRequest {
	character_id: Snowflake;
	version_id?: Snowflake | null;
	greeting_id: Snowflake;
	/** Seeds the new conversation's author's note. */
	author_note?: string;
}

export interface TurnRequest {
	content?: string | null;
	actions?: PlayerAction[];
	/** A list of `sha256:`-formatted resource locators */
	images?: string[];
	swipe_from?: Snowflake | null;
	continue?: boolean;
}

export type ChatEdit =
	/** Edit a message in place */
	| { op: 'edit'; id: Snowflake; content: string }
	/** Move the active path to run through this message. */
	| { op: 'activate'; id: Snowflake };

export interface FullConversation {
	metadata: Conversation;
	messages: ConversationMessage[];
}

export type ChatStreamEvent =
	| { type: 'turn_started'; handle: string; name: string; observers: string[] }
	| { type: 'delta'; content: string }
	| { type: 'widget'; widget: WidgetInstance }
	| { type: 'reasoning'; content: string }
	| { type: 'effect'; effect: CommandEffect }
	| { type: 'message'; message: ConversationMessage }
	| { type: 'done'; next: string | null };
