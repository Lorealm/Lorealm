import type { JsonValue, PromptRole, Snowflake } from './core.ts';
import type { WidgetInstance } from './widgets.ts';

export interface Conversation {
	id: Snowflake;
	user_id: Snowflake;
	/** Null for realm conversations — they have a cast, not one character. */
	character_id: Snowflake | null;
	char_version_id: Snowflake | null;
	/** The realm preset this playthrough was instantiated from, if any. */
	realm_id: Snowflake | null;
	active_leaf_id: Snowflake | null;
	title: string | null;
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
	content: string;
}

export interface SendMessageRequest {
	content: string; // TODO: enrich with multimodal
}

export interface CompletionsRequest {
	content?: string | null;
	target?: Snowflake | null;
}

export type ChatEdit =
	/** Edit a message in place */
	| { op: 'edit'; id: Snowflake; content: string }
	/** Swipe/branch a user message */
	| { op: 'branch_user'; id: Snowflake; content: string }
	/** Move the active path to run through this message. */
	| { op: 'activate'; id: Snowflake };

export interface FullConversation {
	metadata: Conversation;
	messages: ConversationMessage[];
}

export type ChatStreamEvent =
	| { type: 'user_message'; message: ConversationMessage }
	/** A chunk of generated text */
	| { type: 'delta'; content: string }
	| { type: 'widget'; widget: WidgetInstance }
	| { type: 'reasoning'; content: string }
	| { type: 'done'; message: ConversationMessage };
