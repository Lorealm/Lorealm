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
import { messageStateStore } from './messageState.svelte.ts';

export type StreamStatus = 'idle' | 'streaming' | 'error';

export interface StreamState {
	status: StreamStatus;
	message?: string;
}

export interface NewConversationOptions {
	characterId: Snowflake;
	versionId?: Snowflake | null;
	greetings: Greeting[];
}

interface PendingState extends NewConversationOptions {
	selected: number;
}

const TEMP_PREFIX = 'tmp_';
const isTemp = (id: Snowflake) => (id as unknown as string).startsWith(TEMP_PREFIX);

class ChatStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi(); // NOTE: This assumes it's accessed once during page init. Dirty workaround.
		return this._api;
	}

	ensureApi() {
		void this.#api;
	}

	metadataList = $state<Array<ConversationMetadata>>();

	/** Active conversation metadata */
	private _conversation = $state<Conversation | null>(null);

	/** Every message (including branches) of the active conversation */
	private _messages = new SvelteMap<Snowflake, ConversationMessage>();

	private _activeLeafId = $state<Snowflake | null>(null);

	private _pending = $state<PendingState | null>(null);
	private _pendingGreetingId: Snowflake | null = null;

	private _streamingId = $state<Snowflake | null>(null);
	private _streamBuffer = $state('');

	streamState = $state<StreamState>({ status: 'idle' });

	#tempCounter = 0;
	#tempId(): Snowflake {
		return `${TEMP_PREFIX}${this.#tempCounter++}` as unknown as Snowflake;
	}

	get conversation(): Conversation | null {
		return this._conversation;
	}
	get conversationId(): Snowflake | null {
		return this._conversation?.id ?? null;
	}
	/** The character this chat is bound to */
	get characterId(): Snowflake | null {
		return this._conversation?.character_id ?? this._pending?.characterId ?? null;
	}
	get isStreaming(): boolean {
		return this.streamState.status === 'streaming';
	}

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

	siblingInfo(id: Snowflake): { index: number; count: number } {
		if (this._pending && id === this._pendingGreetingId) {
			return { index: this._pending.selected, count: this._pending.greetings.length };
		}
		const msg = this._messages.get(id);
		if (!msg) return { index: 0, count: 1 };
		const sibs = this.#siblingsOf(msg);
		return { index: sibs.findIndex((s) => s.id === id), count: sibs.length };
	}

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

	#descendToLeaf(id: Snowflake): Snowflake {
		let cur = id;
		for (;;) {
			const children = this.#childrenOf(cur);
			if (children.length === 0) return cur;
			cur = children[children.length - 1].id;
		}
	}

	/** Pick a sensible leaf when the server didn't give one. */
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

	async loadList() {
		this.metadataList = await this.#api.chat.list();
	}

	/** Open a persisted conversation */
	async loadConversation(id: Snowflake) {
		this.#resetSession();
		const full = await this.#api.chat.get(id, true);
		this.#ingestFull(full);
	}

	messageById(id: Snowflake): ConversationMessage | undefined {
		return this._messages.get(id);
	}

	/** Open this character's most recent conversation */
	async openLatestForCharacter(characterId: Snowflake): Promise<boolean> {
		const chats = await this.#api.characters.chats(characterId);
		if (chats.length === 0) return false;
		const latest = chats.reduce((a, b) => (a.updated_at >= b.updated_at ? a : b));
		await this.loadConversation(latest.id);

		const leafId = this._activeLeafId;
		if (leafId) messageStateStore.inspect(leafId, latest.id);
		return true;
	}

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

	/** Send a user message */
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
			await this.loadList();
			await this.#runCompletion(null, null); // generate the first assistant reply
			return;
		}

		await this.#runCompletion(content, null);
	}

	/** Continue or regenerate from the current leaf. */
	async regenerate() {
		if (this.isStreaming) return;
		await this.#runCompletion(null, null);
	}

	async commitUserSwipe(target: Snowflake, content: string) {
		if (this.isStreaming) return;
		await this.#runCompletion(content, target);
	}

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
	}

	/** switch to the previous sibling */
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

	#provisionalParent(content: string | null, target: Snowflake | null): Snowflake | null {
		if (target) {
			const t = this._messages.get(target);
			// Some/Some inserts a user sibling first
			return t?.parent ?? null;
		}
		if (content !== null) return this._activeLeafId; // Some/None: under current leaf
		// None/None: regenerate.
		const leaf = this._activeLeafId ? this._messages.get(this._activeLeafId) : undefined;
		if (!leaf) return null;
		return leaf.role === 'user' ? leaf.id : leaf.parent;
	}

	async #runCompletion(content: string | null, target: Snowflake | null) {
		const convId = this.conversationId;
		if (!convId) return; // can't stream without a persisted conversation

		const prevLeaf = this._activeLeafId;
		let provisionalUserId: Snowflake | null = null;

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

			let committedUser: ConversationMessage | null = null;
			let committedAssistant: ConversationMessage | null = null;
			for await (const event of stream as AsyncIterable<ChatStreamEvent>) {
				switch (event.type) {
					case 'delta':
						this._streamBuffer += event.content;
						console.log('delta');
						break;
					case 'user_message':
						committedUser = event.message;
						console.log('user message');
						break;
					case 'done':
						committedAssistant = event.message;
						break;
				}
			}

			// Reconcile locally from the messages the stream carried
			if (provisionalUserId) this._messages.delete(provisionalUserId);
			this._messages.delete(assistantId);
			if (committedUser) this._messages.set(committedUser.id, committedUser);
			if (committedAssistant) this._messages.set(committedAssistant.id, committedAssistant);

			this._activeLeafId = committedAssistant?.id ?? this.#deriveLeaf();
			this._streamingId = null;
			this._streamBuffer = '';
			this.streamState = { status: 'idle' };

			// TODO: think of alternative approachs
			if (committedAssistant) {
				messageStateStore.inspect(committedAssistant?.id, committedAssistant?.conversation_id);
			}
		} catch (e) {
			// Roll the provisional bubbles back so a failed stream leaves no ghosts.
			if (provisionalUserId) this._messages.delete(provisionalUserId);
			this._messages.delete(assistantId);
			this._activeLeafId = prevLeaf;
			this._streamingId = null;
			this._streamBuffer = '';
			this.streamState = { status: 'error', ...extractError(e) };
			console.log(e);
		}
	}

	/** Edit a message's text in place */
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

	/** Route the active path through `id` */
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

	/** Delete a message and its whole subtree. */
	async deleteMessage(id: Snowflake) {
		const convId = this.conversationId;
		const target = convId ? this._messages.get(id) : undefined;
		if (!convId || !target) return;

		// Collect the subtree for removal & rollback.
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

	/** Delete a conversation */
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
