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
import type { PlayerAction, RealmAgent, RealmCast, TurnResult } from '$lib/types/realm.js';
import type { WidgetInstance } from '$lib/types/widgets.js';
import { SvelteMap } from 'svelte/reactivity';
import { messageStateStore } from './messageState.svelte.ts';
import { showError, type ToastAction } from './errorToast.svelte.ts';

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

function scopeMatches(pattern: string, ns: string): boolean {
	return pattern.endsWith('*') ? ns.startsWith(pattern.slice(0, -1)) : ns === pattern;
}

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
	private _streamReasoning = $state('');
	private _streamWidgets = $state<WidgetInstance[]>([]);
	private _streamingConvId: Snowflake | null = null;
	private _cancelling = $state(false);

	private _realmCast = $state<RealmCast | null>(null);
	private _nextAgentId = $state<Snowflake | null>(null);
	private _streamAgentName = $state<string | null>(null);
	private _streamAgentId = $state<Snowflake | null>(null);
	realmAutoRun = $state(true);
	realmDirectorView = $state(false);

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
	get isCancelling(): boolean {
		return this._cancelling;
	}

	get isRealm(): boolean {
		return this._conversation?.realm_id != null;
	}
	get realmCast(): RealmCast | null {
		return this._realmCast;
	}
	/** Whose turn is next, as resolved by the server. */
	get nextRealmAgent(): RealmAgent | null {
		const id = this._nextAgentId;
		if (!this.isRealm || id === null) return null;
		return this._realmCast?.agents.find((a) => a.id === id) ?? null;
	}
	/** Set when the next turn belongs to a player */
	get awaitingRealmPlayer(): RealmAgent | null {
		if (this.isStreaming) return null;
		const next = this.nextRealmAgent;
		return next?.kind === 'player' ? next : null;
	}

	/** The player agent this viewer drives.
	 *  Null while spectating an all-LLM cast. */
	get viewerAgent(): RealmAgent | null {
		if (!this.isRealm) return null;
		return this._realmCast?.agents.find((a) => a.kind === 'player') ?? null;
	}

	/** The source agent a realm message was committed by, via its metadata. */
	#messageAgentId(m: ConversationMessage): Snowflake | null {
		return (m.metadata as { agent_id?: Snowflake } | null)?.agent_id ?? null;
	}

	/** Can `viewer` hear speech carried on `scope`? */
	#canHear(viewer: RealmAgent, scope: string): boolean {
		return viewer.grants.some((g) => scopeMatches(g.channel, scope));
	}

	get visiblePath(): ConversationMessage[] {
		const path = this.activePath;
		const viewer = this.viewerAgent;
		if (this.realmDirectorView || !viewer) return path;

		const agents = this._realmCast?.agents ?? [];
		return path.filter((m) => {
			if (this.#messageAgentId(m) === viewer.id) return true; // own turn
			if (m.content.trim().length === 0) return false; // silent (script) turn
			const src = agents.find((a) => a.id === this.#messageAgentId(m));
			return this.#canHear(viewer, src?.speech_channel ?? 'main');
		});
	}

	/** True while a turn the player cannot hear is streaming */
	get hiddenTurnActive(): boolean {
		if (!this.isStreaming || this.realmDirectorView || !this.viewerAgent) return false;
		const last = this.activePath.at(-1);
		return last != null && !this.visiblePath.some((m) => m.id === last.id);
	}

	get activePath(): ConversationMessage[] {
		const path: ConversationMessage[] = [];
		let cur = this._activeLeafId ? this._messages.get(this._activeLeafId) : undefined;
		while (cur) {
			const msg =
				cur.id === this._streamingId
					? {
							...cur,
							content: this._streamBuffer,
							metadata: {
								widgets: this._streamWidgets,
								reasoning: this._streamReasoning,
								agent_name: this._streamAgentName,
								agent_id: this._streamAgentId
							} as unknown as ConversationMessage['metadata']
						}
					: cur;
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
		if (full.metadata.realm_id != null) {
			try {
				this._realmCast = await this.#api.chat.realm.cast(id);
			} catch (e) {
				this.#handleError(e, { title: 'Failed to load the realm cast' }, 5000);
			}
			await this.#refreshNextRealmAgent();
		}
	}

	/** Re-resolve whose turn it is */
	async #refreshNextRealmAgent() {
		const convId = this.conversationId;
		if (!convId || !this.isRealm) return;
		try {
			this._nextAgentId = (await this.#api.chat.realm.nextTurn(convId)).agent_id;
		} catch {
			this._nextAgentId = null;
		}
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
		this._streamingConvId = null;
		this._cancelling = false;
		this._streamBuffer = '';
		this._streamReasoning = '';
		this._streamWidgets = [];
		this._realmCast = null;
		this._nextAgentId = null;
		this._streamAgentName = null;
		this._streamAgentId = null;
		this.realmAutoRun = true;
		this.streamState = { status: 'idle' };
	}

	/** Send a user message */
	async sendMessage(content: string) {
		if (this.isStreaming) return;

		if (this.isRealm) {
			await this.submitRealmPlayerTurn(content || null, null);
			return;
		}

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
		if (this.isRealm) {
			// Regenerating an assistant leaf re-runs that agent's turn as a
			// sibling; otherwise just advance the world.
			const leaf = this._activeLeafId ? this._messages.get(this._activeLeafId) : undefined;
			if (leaf?.role === 'assistant') await this.runRealmTurn(leaf.id);
			else await this.runRealmTurns();
			return;
		}
		await this.#runCompletion(null, null);
	}

	/** Stop the in-flight generation. */
	async cancelStream() {
		if (!this.isStreaming || this._cancelling) return;
		const convId = this._streamingConvId;
		if (!convId) return;

		// A cancelled realm turn still commits its partial message, so the
		// auto-run loop must not roll on past it.
		this.realmAutoRun = false;
		this._cancelling = true;
		try {
			await this.#api.chat.cancel(convId);
		} catch (e) {
			this._cancelling = false;
			console.warn('Failed to cancel generation', e);
		}
	}

	async commitUserSwipe(target: Snowflake, content: string) {
		if (this.isStreaming) return;
		if (this.isRealm) {
			// Player turns always append at the leaf; there is no
			// alternate-player-turn endpoint (yet).
			this.#handleError(
				new Error('Branching a player turn is not supported yet — regenerate the reply instead.'),
				{ title: 'Not supported in realms' },
				5000
			);
			return;
		}
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
			// new assistant sibling of this one (realms: same agent, new world-line)
			if (this.isRealm) await this.runRealmTurn(id);
			else await this.#runCompletion(null, id);
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
		this._streamingConvId = convId;
		this._streamBuffer = '';
		this._streamReasoning = '';
		this._streamWidgets = [];
		this._cancelling = false;
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
					case 'reasoning':
						this._streamReasoning += event.content;
						break;
					case 'widget':
						this._streamWidgets = [...this._streamWidgets, event.widget];
						console.log('widget', event.widget.widget_id);
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
			this._streamingConvId = null;
			this._cancelling = false;
			this._streamBuffer = '';
			this._streamReasoning = '';
			this._streamWidgets = [];
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
			this._streamingConvId = null;
			this._cancelling = false;
			this._streamBuffer = '';
			this._streamReasoning = '';
			this._streamWidgets = [];
			this.#handleError(e, {}, 5000);
		}
	}

	async runRealmTurn(
		target: Snowflake | null = null
	): Promise<'committed' | 'awaiting_player' | 'stopped'> {
		const convId = this.conversationId;
		if (!convId || !this.isRealm || this.isStreaming) return 'stopped';
		// A player is up: nothing to stream, prompt them instead.
		if (target === null && this.awaitingRealmPlayer) return 'awaiting_player';

		const prevLeaf = this._activeLeafId;
		const assistantParent = target
			? (this._messages.get(target)?.parent ?? null)
			: this._activeLeafId;

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
		this._streamingConvId = convId;
		this._streamBuffer = '';
		this._streamReasoning = '';
		this._streamWidgets = [];
		this._streamAgentName = null;
		this._streamAgentId = null;
		this._cancelling = false;
		this.streamState = { status: 'streaming' };

		try {
			const stream = this.#api.chat.realm.turnStream(convId, target ? { target } : {});

			let committed: ConversationMessage | null = null;
			let awaiting = false;
			for await (const event of stream) {
				switch (event.type) {
					case 'awaiting_player':
						awaiting = true;
						this._nextAgentId = event.agent_id;
						break;
					case 'turn_started':
						this._streamAgentName = event.name;
						this._streamAgentId = event.agent_id;
						break;
					case 'delta':
						this._streamBuffer += event.content;
						break;
					case 'reasoning':
						this._streamReasoning += event.content;
						break;
					case 'widget':
						this._streamWidgets = [...this._streamWidgets, event.widget];
						break;
					case 'action':
						// Thin by design: observers only learn *that* the agent
						// acted; the result lands in the actor's private feed.
						break;
					case 'committed':
						committed = event.message;
						break;
				}
			}

			this._messages.delete(assistantId);
			if (committed) this._messages.set(committed.id, committed);
			this._activeLeafId = committed?.id ?? prevLeaf;
			this.#clearStream();

			if (committed) {
				await this.#refreshNextRealmAgent();
				messageStateStore.inspect(committed.id, committed.conversation_id);
				return 'committed';
			}
			return awaiting ? 'awaiting_player' : 'stopped';
		} catch (e) {
			this._messages.delete(assistantId);
			this._activeLeafId = prevLeaf;
			this.#clearStream(true);
			this.#handleError(e, { title: 'Turn failed' }, 5000);
			return 'stopped';
		}
	}

	/** Keep running LLM turns until a player's turn, a pause, or an error. */
	async runRealmTurns() {
		this.realmAutoRun = true;
		while (this.realmAutoRun && this.isRealm) {
			const result = await this.runRealmTurn();
			if (result !== 'committed') break;
		}
	}

	/** Finish the in-flight turn, then stop advancing. */
	pauseRealm() {
		this.realmAutoRun = false;
	}

	/** The current player's turn: say something, act, or both. */
	async submitRealmPlayerTurn(content: string | null, action: PlayerAction | null): Promise<boolean> {
		const convId = this.conversationId;
		if (!convId || !this.isRealm || this.isStreaming) return false;
		if (!content?.trim() && !action) return false;

		let result: TurnResult;
		try {
			result = await this.#api.chat.realm.playerTurn(convId, { content, action });
		} catch (e) {
			this.#handleError(e, { title: 'Failed to submit turn' }, 5000);
			return false;
		}

		this._messages.set(result.message.id, result.message);
		this._activeLeafId = result.message.id;
		await this.#refreshNextRealmAgent();
		if (this.realmAutoRun) await this.runRealmTurns();
		return true;
	}

	#clearStream(failed = false) {
		this._streamingId = null;
		this._streamingConvId = null;
		this._cancelling = false;
		this._streamBuffer = '';
		this._streamReasoning = '';
		this._streamWidgets = [];
		this._streamAgentName = null;
		this._streamAgentId = null;
		if (!failed) this.streamState = { status: 'idle' };
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
			this.#handleError(
				e,
				{
					title: 'Failed to edit message'
				},
				5000
			);
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
			this.#handleError(
				e,
				{
					title: 'Failed to switch message'
				},
				5000
			);
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

			this.#handleError(
				e,
				{
					title: 'Failed to delete message'
				},
				5000
			);

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

			this.#handleError(
				e,
				{
					title: 'Failed to delete conversation'
				},
				5000
			);

			throw e;
		}
	}

	#handleError(e: unknown, opts?: HandleErrorOptions, timeoutMs?: number) {
		const err = extractError(e);

		this.streamState = {
			status: 'error',
			message: err.message
		};

		showError(err.message, {
			title: opts?.title ?? 'Something went wrong',
			durationMs: timeoutMs ?? 0,
			action: opts?.action
		});
	}
}

type HandleErrorOptions = {
	title?: string;
	action?: ToastAction;
};

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
