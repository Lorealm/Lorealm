import { extractError } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	ChatEdit,
	Conversation,
	ConversationMessage,
	ConversationMetadata,
	FullConversation,
	TurnRequest
} from '$lib/types/conversation.js';
import type { Greeting } from '$lib/types/character.js';
import type { PlayerAction, RealmAgent, RealmCast, TurnEvent } from '$lib/types/realm.js';
import { parseReasoning, parseWidgets, type WidgetInstance } from '$lib/types/widgets.js';
import { SvelteMap } from 'svelte/reactivity';
import { messageStateStore } from './messageState.svelte.ts';
import { type ToastAction } from './errorToast.svelte.ts';
import { showApiError } from './errorActions.js';
import { applyEffect } from '$lib/commands/server.svelte.js';

export interface FeedEntry {
	message: ConversationMessage;
	/** Whether the viewer hears the message itself. */
	audible: boolean;
	/** What else the viewer learns from the turn: events on channels they
	 *  hear, and what their own actions returned. */
	notices: string[];
}

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
	/** Author's note typed before the conversation row exists. */
	authorNote: string;
}

const TEMP_PREFIX = 'tmp_';
const isTemp = (id: Snowflake) => (id as unknown as string).startsWith(TEMP_PREFIX);

/** What a turn's message says about who took it. */
interface TurnMetadata {
	handle?: string;
	channel?: string;
	command?: string;
	events?: TurnEvent[];
	tool_results?: string[];
}

const turnMeta = (m: ConversationMessage) => (m.metadata ?? {}) as TurnMetadata;

/** The channel everybody hears. Any other takes a grant. */
const MAIN_CHANNEL = 'main';

function hears(viewer: RealmAgent, channel: string): boolean {
	if (channel === MAIN_CHANNEL) return true;
	return viewer.grants.some((g) => {
		const scope = g.channel.split(':', 1)[0];
		return scope === '*' || scope === channel;
	});
}

class ChatStore {
	get #api() {
		return useApi();
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

	private _cast = $state<RealmCast | null>(null);
	private _nextHandle = $state<string | null>(null);
	private _streamAgentName = $state<string | null>(null);
	private _streamHandle = $state<string | null>(null);
	/** Keep running turns until it is a player's. */
	autoRun = $state(true);
	/** Show every turn, not only what the player's agent perceives. */
	directorView = $state(false);

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
	/** True while a greeting is on screen but no conversation row exists yet. */
	get isPending(): boolean {
		return this._pending !== null;
	}
	get pendingAuthorNote(): string {
		return this._pending?.authorNote ?? '';
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

	get cast(): RealmCast | null {
		return this._cast;
	}
	get isEnsemble(): boolean {
		const agents = this._cast?.agents;
		if (!agents) return false;
		const players = agents.filter((a) => a.kind === 'player').length;
		return players !== 1 || agents.length !== 2;
	}
	/** Whose turn is next, as resolved by the server. */
	get nextAgent(): RealmAgent | null {
		const handle = this._nextHandle;
		if (handle === null) return null;
		return this._cast?.agents.find((a) => a.handle === handle) ?? null;
	}
	/** Set when the next turn belongs to a player */
	get awaitingPlayer(): RealmAgent | null {
		if (this.isStreaming) return null;
		const next = this.nextAgent;
		return next?.kind === 'player' ? next : null;
	}

	/** The player agent this viewer drives. Null while spectating a cast
	 *  with no player in it. */
	get viewerAgent(): RealmAgent | null {
		return this._cast?.agents.find((a) => a.kind === 'player') ?? null;
	}

	/** Does the viewer hear what this message says? */
	#audible(m: ConversationMessage, viewer: RealmAgent): boolean {
		const meta = turnMeta(m);
		if (meta.handle === viewer.handle) return true;
		if (meta.handle === undefined) return m.content.trim().length > 0 || meta.command != null;
		if (m.id !== this._streamingId && m.content.trim().length === 0) return false;
		return hears(viewer, meta.channel ?? MAIN_CHANNEL);
	}

	get renderedPath(): ConversationMessage[] {
		const viewer = this.viewerAgent;
		if (this.directorView || !viewer) return this.activePath;
		return this.activePath.filter((m) => this.#audible(m, viewer));
	}

	/** What the transcript renders. */
	get renderedFeed(): FeedEntry[] {
		const viewer = this.directorView ? null : this.viewerAgent;
		return this.activePath
			.map((message) => {
				const meta = turnMeta(message);
				const own = !viewer || meta.handle === viewer.handle;
				return {
					message,
					audible: !viewer || this.#audible(message, viewer),
					notices: [
						...(meta.events ?? [])
							.filter((e) => own || hears(viewer, e.channel))
							.map((e) => e.content),
						...(own ? (meta.tool_results ?? []) : [])
					]
				};
			})
			.filter((entry) => entry.audible || entry.notices.length > 0);
	}

	/** The newest message on screen */
	get lastMessage(): ConversationMessage | null {
		return this.renderedPath.at(-1) ?? null;
	}

	/** True while a turn the player cannot hear is streaming */
	get hiddenTurnActive(): boolean {
		if (!this.isStreaming) return false;
		const last = this.activePath.at(-1);
		return last != null && !this.renderedPath.some((m) => m.id === last.id);
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
								handle: this._streamHandle,
								channel: this._cast?.agents.find((a) => a.handle === this._streamHandle)
									?.speech_channel
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

	/**
	 * Fold in a message the server already appended and made the active leaf.
	 *
	 * Used by command runs, which persist their own anchor message rather than
	 * streaming one back.
	 */
	ingestServerMessage(message: ConversationMessage) {
		if (message.conversation_id !== this.conversationId) return;
		this._messages.set(message.id, message);
		this._activeLeafId = message.id;
	}

	async loadList() {
		this.metadataList = await this.#api.chat.list();
	}

	/** Open a persisted conversation */
	async loadConversation(id: Snowflake) {
		this.#resetSession();
		this.#ingestFull(await this.#api.chat.get(id, true));
		await this.#loadCast();
	}

	/** Who is in the conversation, and whose turn it is. */
	async #loadCast() {
		const id = this.conversationId;
		if (!id) return;
		try {
			const [cast, next] = await Promise.all([
				this.#api.chat.cast(id),
				this.#api.chat.nextTurn(id)
			]);
			this._cast = cast;
			this._nextHandle = next.handle;
		} catch (e) {
			this.#handleError(e, { title: 'Failed to load the cast' }, 5000);
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
		this._pending = { ...opts, greetings, selected: 0, authorNote: '' };

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

	/** Drop local traces of a deleted character's chats. */
	forgetCharacter(characterId: Snowflake) {
		if (this.characterId === characterId) this.closeConversation();
		if (this.metadataList)
			this.metadataList = this.metadataList.filter((m) => m.character_id !== characterId);
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
		this._cast = null;
		this._nextHandle = null;
		this._streamAgentName = null;
		this._streamHandle = null;
		this.autoRun = true;
		this.directorView = false;
		this.streamState = { status: 'idle' };
	}

	/** Say something as the player, then let the cast answer. */
	async sendMessage(content: string) {
		if (this.isStreaming) return;

		const p = this._pending;
		if (p) {
			// The greeting on screen becomes the conversation's first turn.
			const greeting = p.greetings[p.selected];
			if (!greeting) return;
			try {
				const full = await this.#api.chat.start({
					character_id: p.characterId,
					version_id: p.versionId,
					greeting_id: greeting.id,
					author_note: p.authorNote
				});
				this.#ingestFull(full);
			} catch (e) {
				this.#handleError(e, { title: 'Failed to start the chat' }, 5000);
				return;
			}
			await Promise.all([this.loadList(), this.#loadCast()]);
		}

		await this.#play({ content });
	}

	/** Trigger one of the player's bound tools as their turn. */
	async act(action: PlayerAction) {
		if (this.isStreaming) return;
		await this.#play({ action });
	}

	/** Replace one of the player's own turns with a new take on it. */
	async commitUserSwipe(target: Snowflake, content: string) {
		if (this.isStreaming) return;
		await this.#play({ content, swipe_from: target });
	}

	async continueMessage(id: Snowflake | null = null) {
		if (this.isStreaming) return;
		const target = id ?? this._activeLeafId;
		if (!target || isTemp(target)) return;
		if (this._messages.get(target)?.role !== 'assistant') return;
		await this.#runTurn({ swipe_from: target, continue: true });
	}

	/** Run a single turn: whoever is next, unless that is a player. */
	async step() {
		if (this.isStreaming || this.awaitingPlayer) return;
		await this.#runTurn({});
	}

	async play() {
		if (this.isStreaming) return;
		await this.#play({});
	}

	pause() {
		this.autoRun = false;
	}

	async cancelStream() {
		if (!this.isStreaming || this._cancelling) return;
		const convId = this._streamingConvId;
		if (!convId) return;

		// A cancelled turn still commits what it had
		this.autoRun = false;
		this._cancelling = true;
		try {
			await this.#api.chat.cancel(convId);
		} catch (e) {
			this._cancelling = false;
			console.warn('Failed to cancel generation', e);
		}
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
		// Past the last take: whoever took this turn takes it again.
		if (msg.role === 'assistant') await this.#runTurn({ swipe_from: id });
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

	/** Make a request, then keep the cast going until it is a player's turn. */
	async #play(request: TurnRequest) {
		this.autoRun = true;
		let next = request;
		while (await this.#runTurn(next)) {
			const up = this.nextAgent;
			if (!this.autoRun || !up || up.kind === 'player') break;
			next = {};
		}
	}

	#provisional(parent: Snowflake | null, role: 'user' | 'assistant', content: string): Snowflake {
		const id = this.#tempId();
		this._messages.set(id, {
			id,
			conversation_id: this.conversationId!,
			parent,
			role,
			content,
			order: Number.MAX_SAFE_INTEGER,
			metadata: null,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		});
		return id;
	}

	async #runTurn(request: TurnRequest): Promise<boolean> {
		const convId = this.conversationId;
		if (!convId) return false; // can't stream without a persisted conversation

		// The last leaf the server is known to hold: where a failure lands.
		let settled = this._activeLeafId;
		const target = request.swipe_from ? this._messages.get(request.swipe_from) : undefined;
		const continued = request.continue ? target : undefined;
		const speaks = request.content != null || request.action != null;

		// The player's words show at once; the server's copy replaces them.
		let pendingUser: Snowflake | null = null;
		if (request.content) {
			pendingUser = this.#provisional(
				target ? target.parent : this._activeLeafId,
				'user',
				request.content
			);
			this._activeLeafId = pendingUser;
		}
		// The bubble a streaming turn fills, made when the turn starts.
		let bubble: Snowflake | null = null;
		let acted = false;

		this._streamBuffer = continued?.content ?? '';
		this._streamReasoning = continued ? parseReasoning(continued.metadata) : '';
		this._streamWidgets = continued ? parseWidgets(continued.metadata) : [];
		this._streamingId = continued?.id ?? null;
		this._streamingConvId = convId;
		this._cancelling = false;
		this.streamState = { status: 'streaming' };

		const dropProvisional = () => {
			if (pendingUser) this._messages.delete(pendingUser);
			if (bubble) this._messages.delete(bubble);
			pendingUser = bubble = null;
		};

		try {
			for await (const event of this.#api.chat.turn(convId, request)) {
				switch (event.type) {
					case 'turn_started':
						this._streamAgentName = event.name;
						this._streamHandle = event.handle;
						if (!continued) {
							// Another take sits beside its target; a new turn under the leaf.
							const beside = target && !speaks ? target.parent : this._activeLeafId;
							bubble = this.#provisional(beside, 'assistant', '');
							this._activeLeafId = this._streamingId = bubble;
						}
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
					case 'effect':
						applyEffect(event.effect);
						break;
					case 'message': {
						const message = event.message;
						if (turnMeta(message).handle === this._streamHandle) {
							acted = true;
							this._streamingId = null;
							messageStateStore.inspect(message.id, message.conversation_id);
						}
						dropProvisional();
						this._messages.set(message.id, message);
						// A continuation rewrote a message in place; the leaf stays put.
						if (message.id !== continued?.id) settled = message.id;
						this._activeLeafId = settled;
						break;
					}
					case 'done':
						this._nextHandle = event.next;
						break;
				}
			}
			dropProvisional();
			this._activeLeafId = settled;
			this.#clearStream();
			return acted;
		} catch (e) {
			// Roll the provisional bubbles back so a failed stream leaves no
			// ghosts. Whatever the server did persist stays.
			dropProvisional();
			this._activeLeafId = settled;
			this.#clearStream(true);
			this.#handleError(e, {}, 5000);
			return false;
		}
	}

	#clearStream(failed = false) {
		this._streamingId = null;
		this._streamingConvId = null;
		this._cancelling = false;
		this._streamBuffer = '';
		this._streamReasoning = '';
		this._streamWidgets = [];
		this._streamAgentName = null;
		this._streamHandle = null;
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

	/** Held until `start` creates the row; there is nothing to PATCH yet. */
	setPendingAuthorNote(note: string) {
		if (this._pending) this._pending.authorNote = note;
	}

	async setAuthorNote(id: Snowflake, note: string) {
		const previous = this._conversation;
		if (previous?.id === id) this._conversation = { ...previous, author_note: note };

		try {
			await this.#api.chat.setAuthorNote(id, note);
		} catch (e) {
			const current = this._conversation;
			if (previous && current?.id === id) {
				this._conversation = { ...current, author_note: previous.author_note };
			}
			showApiError(e, { title: 'Failed to save the author’s note' });
		}
	}

	#handleError(e: unknown, opts?: HandleErrorOptions, timeoutMs?: number) {
		const err = extractError(e);

		this.streamState = {
			status: 'error',
			message: err.message
		};

		showApiError(e, {
			title: opts?.title,
			durationMs: timeoutMs ?? 0,
			actions: opts?.actions
		});
	}
}

type HandleErrorOptions = {
	title?: string;
	actions?: ToastAction[];
};

export const chatStore = new ChatStore();
