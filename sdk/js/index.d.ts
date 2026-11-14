/**
 * Types for the frontend of a Lorealm plugin.
 *
 * A plugin's frontend fills slots in the client, listed in its manifest under
 * `[[frontend.slots]]`. Each slot is one of two kinds:
 *
 * - A **frame** (the default): a self-contained HTML file shown in a sandboxed
 *   frame. Its scripts find the API on `window.lorealm`; see
 *   `@lorealm/plugin/frame`.
 * - An **unsafe** slot (`unsafe = true`): an ES module that runs in the client
 *   itself and exports a {@link Mount} function, which is handed the same API.
 *
 * Either way, the API is the frontend's only way to reach anything: it talks
 * to the plugin's own backend, and the backend does the rest.
 *
 * This package has no code. The client provides the API at runtime; this only
 * describes it.
 *
 * @packageDocumentation
 */

/** The ID of anything the server stores. */
export type Snowflake = string;

export type JsonValue =
	null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

/** Who wrote a message. */
export type MessageRole = 'system' | 'user' | 'assistant';

/** A message a slot is attached to. */
export interface SlotMessage {
	id: Snowflake;
	role: MessageRole;
	content: string;
	/**
	 * When the message last changed, as an ISO 8601 timestamp.
	 */
	updated_at: string;
}

/** What every slot tells its plugin. */
export interface SlotContext {
	/** The conversation the user has open, or `null` outside a conversation. */
	conversation: Snowflake | null;
	/** The message the slot sits under, for a `message` slot; otherwise `null`. */
	message: SlotMessage | null;
}

/**
 * A panel in the inspector. It fills the panel's height, and the user can
 * collapse it.
 */
export interface InspectorContext extends SlotContext {
	message: null;
}

/** A strip above the chat's input box, as tall as its content. */
export interface ComposerContext extends SlotContext {
	message: null;
}

/**
 * A strip under a message, as tall as its content.
 *
 * It has two differing behaviors depending on the `mode`
 * the manifest declares:
 * `latest` (default): The frame is only attached to the latest message
 * `every`: The frame is attached to every message. Be wary of this setting
 * as it may cause performance issues.
 */
export interface MessageContext extends SlotContext {
	conversation: Snowflake;
	message: SlotMessage;
}

export interface SlotContexts {
	inspector: InspectorContext;
	composer: ComposerContext;
	message: MessageContext;
}

/** The name of a slot -- `slot = "..."` in the manifest. */
export type SlotKind = keyof SlotContexts;

/** Options for {@link LorealmApi.call}. */
export interface CallOptions {
	/**
	 * Whether this action should block the user from generating messages for
	 * the duration of this call.
	 *
	 * @defaultValue `false`
	 */
	blocking?: boolean;
}

/** Stops a listener. Calling it more than once does nothing. */
export type Unsubscribe = () => void;

/**
 * What a slot gets from the client: its context, and a channel to the plugin's
 * backend.
 *
 * @typeParam Context - What the slot tells the plugin. Name the slot's own
 *   context, such as {@link MessageContext}, for its exact shape.
 */
export interface LorealmApi<Context extends SlotContext = SlotContext> {
	/**
	 * Where the slot is right now. It changes as the user moves around, such
	 * as when they open another conversation or edit the message.
	 */
	readonly context: Context;

	/**
	 * Calls the plugin's backend, which receives it as a frontend call.
	 *
	 * @param name - The name of the message you're sending.
	 * @param args - An arbitrary JSON payload. Leaving it out sends `null`.
	 * @returns What the backend returned.
	 * @throws An `Error` when the backend returns an error or can't be reached.
	 *
	 * @example
	 * ```ts
	 * const words = await lorealm.call('count_words', { text: 'hello there' });
	 * ```
	 */
	call(name: string, args?: JsonValue, options?: CallOptions): Promise<JsonValue>;

	/**
	 * Sets up am event listener. Whenever the backend sends a message to the
   * frontend, it's sent here.
	 *
	 * @returns A function that stops listening.
	 *
	 * @example
	 * ```ts
	 * lorealm.on('count', (count) => {
	 *   label.textContent = `${count} replies`;
	 * });
	 * ```
	 */
	on(name: string, listener: (payload: JsonValue) => void): Unsubscribe;

	/**
	 * Listens for changes to {@link LorealmApi.context}.
	 *
	 * @returns A function that stops listening.
	 */
	onContext(listener: (context: Context) => void): Unsubscribe;
}

/**
 * What an unsafe slot's module exports as `mount`.
 *
 * @typeParam Context - The slot's context, such as {@link InspectorContext}.
 *
 * @example
 * ```ts
 * import type { InspectorContext, Mount } from '@lorealm/plugin';
 *
 * export const mount: Mount<InspectorContext> = (element, lorealm) => {
 *   const label = element.appendChild(document.createElement('p'));
 *   const stop = lorealm.on('count', (count) => {
 *     label.textContent = `${count} replies`;
 *   });
 *   return stop;
 * };
 * ```
 */
export type Mount<Context extends SlotContext = SlotContext> = (
	element: HTMLElement,
	lorealm: LorealmApi<Context>
) => void | (() => void);

/** The shape of an unsafe slot's module. */
export interface PluginModule<Context extends SlotContext = SlotContext> {
	mount: Mount<Context>;
}
