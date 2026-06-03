import type { Snowflake } from './core.ts';

/** Arbitrary JSON (`serde_json::Value`). */
export type JsonValue =
	| null
	| boolean
	| number
	| string
	| JsonValue[]
	| { [key: string]: JsonValue };

/**
 * Placeholder for the Rust `ChatRole` enum, which isn't included in the
 * snippet. Adjust to match its actual variants / serde renaming.
 */
export type ChatRole = 'user' | 'assistant' | 'system';

// ---------------------------------------------------------------------------
// Core records (Serialize + Deserialize)
// ---------------------------------------------------------------------------

export interface Conversation {
	id: Snowflake;
	user_id: Snowflake;
	character_id: Snowflake;
	char_version_id: Snowflake;
	active_leaf_id: Snowflake | null;
	title: string | null;
	created_at: string;
	updated_at: string;
}

export interface ConversationMessage {
	id: Snowflake;
	conversation_id: Snowflake;
	parent: Snowflake | null;
	role: ChatRole;
	content: string;
	order: number;
	metadata: JsonValue;
	created_at: string;
	updated_at: string;
}

export interface ConversationMetadata {
	id: Snowflake;
	title: string | null;
	length: number;
	created_at: string;
	updated_at: string;
}

// ---------------------------------------------------------------------------
// Requests (Deserialize)
// ---------------------------------------------------------------------------

export interface StartChatRequest {
	character_id: Snowflake;
	// `Option<Snowflake>` with no #[serde(default)] — serde still treats a
	// missing field as None, so it's optional and may be null.
	version_id?: Snowflake | null;
	greeting_id: Snowflake;
	content: string;
}

export interface SendMessageRequest {
	content: string; // TODO: enrich with multimodal
}

export interface CompletionsRequest {
	// Both fields are `#[serde(default)] Option<_>` — absent or null.
	content?: string | null;
	target?: Snowflake | null;
}

/**
 * `ChatEdit` — internally tagged on `op`, variants renamed to snake_case.
 */
export type ChatEdit =
	/** Edit a message in place (no new branch). */
	| { op: 'edit'; id: Snowflake; content: string }
	/**
	 * Swipe/branch a user message: new sibling, becomes active leaf.
	 * Client then calls /completions to generate the reply.
	 */
	| { op: 'branch_user'; id: Snowflake; content: string }
	/** Move the active path to run through this message. */
	| { op: 'activate'; id: Snowflake };

// ---------------------------------------------------------------------------
// Responses (Serialize)
// ---------------------------------------------------------------------------

export interface FullConversation {
	metadata: Conversation;
	messages: ConversationMessage[];
}

/**
 * `ChatStreamEvent` — internally tagged on `type`, variants renamed to
 * snake_case.
 */
export type ChatStreamEvent =
	/** A chunk of generated text. Client appends to a provisional bubble. */
	| { type: 'delta'; content: string }
	/**
	 * Terminal: the persisted assistant message, carrying its real id so the
	 * client can reconcile the provisional bubble and fetch branch_info.
	 */
	| { type: 'done'; message: ConversationMessage };
