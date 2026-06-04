import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	ChatEdit,
	ChatStreamEvent,
	Conversation,
	ConversationMessage,
	ConversationMetadata,
	FullConversation
} from '$lib/types/conversation.js';
import type { Greeting } from '$lib/types/character.js';
import { SvelteMap } from 'svelte/reactivity';

// ---------------------------------------------------------------------------
// A conversation is a TREE of messages, not a flat list. Each message points at
// its `parent`; `active_leaf_id` on the conversation names the currently visible
// leaf. The displayed chat is the chain from the root greeting down to that leaf
// (walk `parent` up, reverse). Anything sharing a parent is a "slide" — the
// chevrons step between siblings.
//
// Three things shape this store:
//
//  1. Streaming, not request/response. `completions` is an SSE stream of
//     `delta` chunks terminated by `done`. We paint a provisional assistant
//     bubble from the deltas for instant feedback, then reconcile against the
//     server on `done`. Live token text lives in ONE `$state` string
//     (`_streamBuffer`) rather than rewriting a map entry per token — the
//     active-path getter splices it in for the streaming message.
//
//  2. The 4-way completions contract (content × target) is the only way new
//     turns are made. We expose one private `#runCompletion` and four thin
//     public wrappers so each row of the contract has exactly one entry point:
//       Some/None  → normal turn          (sendMessage)
//       Some/Some  → user-message swipe    (commitUserSwipe)
//       None/Some  → assistant swipe       (swipeNext at last slide / regenerate of a sibling)
//       None/None  → continue / regenerate (regenerate)
//
//  3. A conversation does not exist until the first user message. Before that
//     we run in a "pending" mode: a synthetic greeting bubble is shown with a
//     null conversation id, and the first `sendMessage` calls `start` (which
//     persists greeting + user message together) before streaming the reply.
//
// Reconciliation choice: on `done` we refetch `messages.list` (the full tree)
// rather than fabricating the committed user message from `done.message.parent`.
// `edit`/`activate` PUTs are treated as fire-and-persist — the local tree is the
// source of truth between loads, and a (re)load is authoritative.
// ---------------------------------------------------------------------------

export type StreamStatus = 'idle' | 'streaming' | 'error';

export interface StreamState {
	status: StreamStatus;
	/** Server diagnostic, surfaced verbatim on `error`. */
	message?: string;
}

/** What the host hands us to open a brand-new (not-yet-persisted) conversation.
 *  A character may define several greetings; the author picks one (by swiping
 *  the greeting bubble) before the first send, and the selected greeting's id is
 *  what `start` commits. */
export interface NewConversationOptions {
	characterId: Snowflake;
	versionId?: Snowflake | null;
	greetings: Greeting[];
}

/** Internal pending state: the options plus which greeting is currently shown. */
interface PendingState extends NewConversationOptions {
	selected: number;
}

const TEMP_PREFIX = 'tmp_';
const isTemp = (id: Snowflake) => (id as unknown as string).startsWith(TEMP_PREFIX);

class ChatStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi(); // accessed once during page init
		return this._api;
	}

	/** Resolve the lazy `#api` now, while a component context is live. `useApi()`
	 *  calls `getContext`, which Svelte only permits during component
	 *  initialisation — so call this from a component's top-level `<script>` (the
	 *  view that hosts the chat), never from an event handler. Idempotent; the
	 *  resolved client is cached for the rest of the session. */
	ensureApi() {
		void this.#api;
	}

	/** Listing-only metadata for the sidebar. Does not carry messages. */
	metadataList = $state<Array<ConversationMetadata>>();

	/** Active conversation metadata. `null` while in pending (pre-`start`) mode. */
	private _conversation = $state<Conversation | null>(null);

	/** Every message of the active conversation, the whole tree keyed by id.
	 *  Provisional (temp-id) bubbles live here too during a stream. */
	private _messages = new SvelteMap<Snowflake, ConversationMessage>();

	/** The visible leaf. Defines the active path. */
	private _activeLeafId = $state<Snowflake | null>(null);

	/** Pending (pre-`start`) state. `null` once the chat is persisted or closed. */
	private _pending = $state<PendingState | null>(null);
	/** Temp id of the synthetic greeting bubble shown in pending mode. */
	private _pendingGreetingId: Snowflake | null = null;

	/** Live token text for the in-flight assistant bubble. Spliced into the
	 *  active path by id so we don't churn the message map per token. */
	private _streamingId = $state<Snowflake | null>(null);
	private _streamBuffer = $state('');

	streamState = $state<StreamState>({ status: 'idle' });

	#tempCounter = 0;
	#tempId(): Snowflake {
		return `${TEMP_PREFIX}${this.#tempCounter++}` as unknown as Snowflake;
	}

	/* --- accessors -------------------------------------------------------- */

	get conversation(): Conversation | null {
		return this._conversation;
	}
	get conversationId(): Snowflake | null {
		return this._conversation?.id ?? null;
	}
	/** The character this chat is bound to — for avatar src, name lookups, etc.
	 *  Available in both pending and persisted modes. */
	get characterId(): Snowflake | null {
		return this._conversation?.character_id ?? this._pending?.characterId ?? null;
	}
	get isStreaming(): boolean {
		return this.streamState.status === 'streaming';
	}

	/** The ordered, visible messages (root → leaf). The streaming bubble carries
	 *  the live buffer instead of its (empty) stored content. */
	get activePath(): ConversationMessage[] {
		const path: ConversationMessage[] = [];
		let cur = this._activeLeafId ? this._messages.get(this._activeLeafId) : undefined;
		while (cur) {
			const msg = cur.id === this._streamingId ? { ...cur, content: this._streamBuffer } : cur;
			path.unshift(msg);
			cur = cur.parent ? this._messages.get(cur.parent) : undefined;
		}
		return path;
	}

	/** Slide position of a message among its siblings — drives the chevrons.
	 *  `count === 1` means no chevrons; `index === count - 1` is the last slide
	 *  (chevron-right generates from there). In pending mode the greeting bubble's
	 *  "siblings" are the character's greetings. */
	siblingInfo(id: Snowflake): { index: number; count: number } {
		if (this._pending && id === this._pendingGreetingId) {
			return { index: this._pending.selected, count: this._pending.greetings.length };
		}
		const msg = this._messages.get(id);
		if (!msg) return { index: 0, count: 1 };
		const sibs = this.#siblingsOf(msg);
		return { index: sibs.findIndex((s) => s.id === id), count: sibs.length };
	}

	/* --- tree helpers ----------------------------------------------------- */

	#siblingsOf(msg: ConversationMessage): ConversationMessage[] {
		return [...this._messages.values()]
			.filter((m) => m.parent === msg.parent)
			.sort((a, b) => a.order - b.order);
	}

	#childrenOf(id: Snowflake): ConversationMessage[] {
		return [...this._messages.values()]
			.filter((m) => m.parent === id)
			.sort((a, b) => a.order - b.order);
	}

	/** Descend from a node to a leaf, following the latest child at each step.
	 *  Used when switching slides: a sibling may already own a subtree, and we
	 *  want to land on its tip rather than mid-branch. */
	#descendToLeaf(id: Snowflake): Snowflake {
		let cur = id;
		for (;;) {
			const children = this.#childrenOf(cur);
			if (children.length === 0) return cur;
			cur = children[children.length - 1].id;
		}
	}

	/** Pick a sensible leaf when the server didn't give one (e.g. straight after
	 *  `start`): the message with the highest `order`. */
	#deriveLeaf(): Snowflake | null {
		let leaf: ConversationMessage | null = null;
		for (const m of this._messages.values()) {
			if (!leaf || m.order > leaf.order) leaf = m;
		}
		return leaf?.id ?? null;
	}

	#ingestFull(full: FullConversation) {
		this._messages.clear();
		for (const m of full.messages) this._messages.set(m.id, m);
		this._conversation = full.metadata;
		this._activeLeafId = full.metadata.active_leaf_id ?? this.#deriveLeaf();
		this._pending = null;
	}

	/* --- loading / session ------------------------------------------------ */

	async loadList() {
		this.metadataList = await this.#api.chat.list();
	}

	/** Open a persisted conversation for viewing/editing. */
	async loadConversation(id: Snowflake) {
		this.#resetSession();
		const full = await this.#api.chat.get(id, true);
		this.#ingestFull(full);
	}

	/** Open this character's most recent conversation. Returns `true` if one
	 *  existed (and was loaded), `false` if the character has no chats yet — the
	 *  caller decides how to begin a fresh one. ISO timestamps sort lexically. */
	async openLatestForCharacter(characterId: Snowflake): Promise<boolean> {
		const chats = await this.#api.characters.chats(characterId);
		if (chats.length === 0) return false;
		const latest = chats.reduce((a, b) => (a.updated_at >= b.updated_at ? a : b));
		await this.loadConversation(latest.id);
		return true;
	}

	/** Enter pending mode for a not-yet-persisted chat. Shows the first greeting
	 *  (by `order`); the author can swipe the bubble to pick another before the
	 *  first send. Does nothing if the character has no greetings — `start`
	 *  requires one. */
	beginNewConversation(opts: NewConversationOptions) {
		this.#resetSession();
		if (opts.greetings.length === 0) return;
		const greetings = [...opts.greetings].sort((a, b) => a.order - b.order);
		this._pending = { ...opts, greetings, selected: 0 };

		const id = this.#tempId();
		this._pendingGreetingId = id;
		this._messages.set(id, {
			id,
			conversation_id: id,
			parent: null,
			role: 'assistant',
			content: greetings[0].text,
			order: 0,
			metadata: null,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		});
		this._activeLeafId = id;
	}

	/** Switch the pending greeting bubble to `index`, clamped. */
	#showGreeting(index: number) {
		const p = this._pending;
		const id = this._pendingGreetingId;
		if (!p || !id) return;
		const next = Math.max(0, Math.min(index, p.greetings.length - 1));
		if (next === p.selected) return;
		p.selected = next;
		const msg = this._messages.get(id);
		if (msg) this._messages.set(id, { ...msg, content: p.greetings[next].text });
	}

	closeConversation() {
		this.#resetSession();
	}

	#resetSession() {
		this._messages.clear();
		this._conversation = null;
		this._activeLeafId = null;
		this._pending = null;
		this._pendingGreetingId = null;
		this._streamingId = null;
		this._streamBuffer = '';
		this.streamState = { status: 'idle' };
	}

	/* --- sending / generation --------------------------------------------- */

	/** Send a user message. In pending mode this first `start`s the conversation
	 *  (persisting greeting + user message), then streams the reply. Otherwise
	 *  it's a normal turn (Some / None). */
	async sendMessage(content: string) {
		if (this.isStreaming) return;

		if (this._pending) {
			const p = this._pending;
			const greeting = p.greetings[p.selected];
			if (!greeting) return;
			const full = await this.#api.chat.start({
				character_id: p.characterId,
				version_id: p.versionId,
				greeting_id: greeting.id,
				content
			});
			this.#ingestFull(full);
			await this.loadList(); // the new conversation now belongs in the sidebar
			await this.#runCompletion(null, null); // generate the first assistant reply
			return;
		}

		await this.#runCompletion(content, null);
	}

	/** Continue / regenerate from the current leaf (None / None). */
	async regenerate() {
		if (this.isStreaming) return;
		await this.#runCompletion(null, null);
	}

	/** Commit a user-message swipe with new text (Some / Some): a new user
	 *  sibling of `target`, then stream the reply. The UI calls this when the
	 *  author edits a user bubble into a new branch. */
	async commitUserSwipe(target: Snowflake, content: string) {
		if (this.isStreaming) return;
		await this.#runCompletion(content, target);
	}

	/** Chevron-right. Within a run of slides it just switches the active sibling.
	 *  At the last slide it generates a new one: assistant messages stream a new
	 *  sibling; user messages can't (they need text) — the UI should open its
	 *  editor and call `commitUserSwipe` instead, so we no-op here. */
	async swipeNext(id: Snowflake) {
		if (this._pending && id === this._pendingGreetingId) {
			this.#showGreeting(this._pending.selected + 1);
			return;
		}
		const msg = this._messages.get(id);
		if (!msg) return;
		const sibs = this.#siblingsOf(msg);
		const idx = sibs.findIndex((s) => s.id === id);

		if (idx < sibs.length - 1) {
			await this.activate(sibs[idx + 1].id);
			return;
		}
		if (this.isStreaming || isTemp(id)) return;
		if (msg.role === 'assistant') {
			await this.#runCompletion(null, id); // new assistant sibling of this one
		}
		// user-message last slide: caller handles via commitUserSwipe
	}

	/** Chevron-left: switch to the previous sibling (clamped at the first). */
	async swipePrev(id: Snowflake) {
		if (this._pending && id === this._pendingGreetingId) {
			this.#showGreeting(this._pending.selected - 1);
			return;
		}
		const msg = this._messages.get(id);
		if (!msg) return;
		const sibs = this.#siblingsOf(msg);
		const idx = sibs.findIndex((s) => s.id === id);
		if (idx > 0) await this.activate(sibs[idx - 1].id);
	}

	/** Where the in-flight assistant bubble attaches, per the completions contract. */
	#provisionalParent(content: string | null, target: Snowflake | null): Snowflake | null {
		if (target) {
			const t = this._messages.get(target);
			// Some/Some inserts a user sibling first (handled by the caller); both
			// cases ultimately sit under the target's parent.
			return t?.parent ?? null;
		}
		if (content !== null) return this._activeLeafId; // Some/None: under current leaf
		// None/None: regenerate. If the leaf is a user turn, the reply hangs off
		// it; if it's an assistant turn, we want a sibling (its parent).
		const leaf = this._activeLeafId ? this._messages.get(this._activeLeafId) : undefined;
		if (!leaf) return null;
		return leaf.role === 'user' ? leaf.id : leaf.parent;
	}

	/** The single streaming path behind every turn. Paints provisional bubble(s),
	 *  consumes the SSE stream into `_streamBuffer`, then reconciles on `done`. */
	async #runCompletion(content: string | null, target: Snowflake | null) {
		const convId = this.conversationId;
		if (!convId) return; // can't stream without a persisted conversation

		const prevLeaf = this._activeLeafId;
		let provisionalUserId: Snowflake | null = null;

		// Some content => a user message is committed too; show it immediately.
		let assistantParent = this.#provisionalParent(content, target);
		if (content !== null) {
			provisionalUserId = this.#tempId();
			this._messages.set(provisionalUserId, {
				id: provisionalUserId,
				conversation_id: convId,
				parent: assistantParent,
				role: 'user',
				content,
				order: Number.MAX_SAFE_INTEGER,
				metadata: null,
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString()
			});
			assistantParent = provisionalUserId;
		}

		const assistantId = this.#tempId();
		this._messages.set(assistantId, {
			id: assistantId,
			conversation_id: convId,
			parent: assistantParent,
			role: 'assistant',
			content: '',
			order: Number.MAX_SAFE_INTEGER,
			metadata: null,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		});

		this._activeLeafId = assistantId;
		this._streamingId = assistantId;
		this._streamBuffer = '';
		this.streamState = { status: 'streaming' };

		try {
			const stream = this.#api.chat.completions(convId, {
				content: content ?? undefined,
				target: target ?? undefined
			});
			let done: ConversationMessage | null = null;
			for await (const event of stream as AsyncIterable<ChatStreamEvent>) {
				if (event.type === 'delta') {
					this._streamBuffer += event.content;
				} else {
					done = event.message;
				}
			}

			// Drop provisional bubbles and take the authoritative tree. The refetch
			// resolves the committed user message's real id/order, which `done`
			// alone doesn't carry.
			if (provisionalUserId) this._messages.delete(provisionalUserId);
			this._messages.delete(assistantId);

			const msgs = await this.#api.chat.messages.list(convId);
			this._messages.clear();
			for (const m of msgs) this._messages.set(m.id, m);
			this._activeLeafId = done?.id ?? this.#deriveLeaf();
			this._streamingId = null;
			this._streamBuffer = '';
			this.streamState = { status: 'idle' };
		} catch (e) {
			// Roll the provisional bubbles back so a failed stream leaves no ghosts.
			if (provisionalUserId) this._messages.delete(provisionalUserId);
			this._messages.delete(assistantId);
			this._activeLeafId = prevLeaf;
			this._streamingId = null;
			this._streamBuffer = '';
			this.streamState = { status: 'error', ...extractError(e) };
		}
	}

	/* --- structural edits (no new generation) ----------------------------- */

	/** Edit a message's text in place — no new branch (op `edit`). Optimistic,
	 *  rolls back on failure. */
	async editMessage(id: Snowflake, content: string) {
		const convId = this.conversationId;
		const msg = convId ? this._messages.get(id) : undefined;
		if (!convId || !msg) return;

		const prev = msg.content;
		this._messages.set(id, { ...msg, content });
		try {
			await this.#api.chat.messages.edit(convId, { op: 'edit', id, content } satisfies ChatEdit);
		} catch (e) {
			this._messages.set(id, { ...msg, content: prev });
			this.streamState = { status: 'error', ...extractError(e) };
		}
	}

	/** Route the active path through `id` (op `activate`). We descend to a leaf
	 *  locally for an instant view switch and persist the choice; rolls the leaf
	 *  back on failure. */
	async activate(id: Snowflake) {
		const convId = this.conversationId;
		if (!convId || !this._messages.has(id)) return;

		const prevLeaf = this._activeLeafId;
		this._activeLeafId = this.#descendToLeaf(id);
		if (isTemp(id)) return; // provisional bubble, nothing to persist yet
		try {
			await this.#api.chat.messages.edit(convId, { op: 'activate', id } satisfies ChatEdit);
		} catch (e) {
			this._activeLeafId = prevLeaf;
			this.streamState = { status: 'error', ...extractError(e) };
		}
	}

	/** Delete a message and its whole subtree. Optimistic; restores on failure.
	 *  If the active leaf was inside the removed subtree, it falls back to the
	 *  deleted node's parent (descended to a leaf). */
	async deleteMessage(id: Snowflake) {
		const convId = this.conversationId;
		const target = convId ? this._messages.get(id) : undefined;
		if (!convId || !target) return;

		// Collect the subtree (target + all descendants) for removal & rollback.
		const removed: ConversationMessage[] = [];
		const stack = [id];
		while (stack.length) {
			const cur = stack.pop()!;
			const m = this._messages.get(cur);
			if (!m) continue;
			removed.push(m);
			for (const c of this.#childrenOf(cur)) stack.push(c.id);
		}

		const prevLeaf = this._activeLeafId;
		const leafRemoved = removed.some((m) => m.id === prevLeaf);
		for (const m of removed) this._messages.delete(m.id);
		if (leafRemoved) {
			this._activeLeafId = target.parent ? this.#descendToLeaf(target.parent) : this.#deriveLeaf();
		}

		try {
			await this.#api.chat.messages.delete(convId, id);
		} catch (e) {
			for (const m of removed) this._messages.set(m.id, m);
			this._activeLeafId = prevLeaf;
			this.streamState = { status: 'error', ...extractError(e) };
			throw e;
		}
	}

	/* --- conversation-level ----------------------------------------------- */

	/** Delete a conversation. Optimistic across the active session + list. */
	async deleteConversation(id: Snowflake) {
		const prevList = this.metadataList ? [...this.metadataList] : undefined;
		const wasActive = this.conversationId === id;

		if (this.metadataList) this.metadataList = this.metadataList.filter((m) => m.id !== id);
		if (wasActive) this.#resetSession();

		try {
			await this.#api.chat.delete(id);
		} catch (e) {
			if (prevList) this.metadataList = prevList;
			throw e;
		}
	}
}

/** Unwrap an http rejection into a clean `{ message, code }`. Mirrors the
 *  transport's `ApiError`; the object/string branches catch raw rejections that
 *  don't go through it. */
function extractError(e: unknown): { message: string } {
	if (e instanceof Error) return { message: e.message };
	if (e && typeof e === 'object') {
		const err = e as Record<string, unknown>;
		const message = typeof err.message === 'string' ? err.message : JSON.stringify(err);
		return { message };
	}
	return { message: String(e) };
}

export const chatStore = new ChatStore();
