import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Conversation } from '$lib/types/conversation.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	ActionSpec,
	AgentGrant,
	AgentObservation,
	CreateAgentRequest,
	PlayerAction,
	Realm,
	RealmAgent,
	RealmCast,
	RealmEvent,
	RealmSummary,
	StateSlice,
	TurnResult
} from '$lib/types/realm.js';
import type { WidgetInstance } from '$lib/types/widgets.js';
import { showError } from './errorToast.svelte.ts';

export type RunStatus = 'idle' | 'streaming' | 'error';

/** The in-flight LLM turn, accumulated from the turn stream. */
export interface LiveTurn {
	agentId: Snowflake;
	name: string;
	text: string;
	reasoning: string;
	widgets: WidgetInstance[];
	actions: Array<{ name: string; is_error: boolean }>;
}

/** What the inspector knows about one agent. */
export interface AgentInspection {
	agentId: Snowflake;
	grants: AgentGrant[];
	actions: ActionSpec[];
	/** Playthrough-only: the agent's grant-filtered pool at the active leaf. */
	state: StateSlice;
	/** Playthrough-only: the agent's private feed. */
	observations: AgentObservation[];
}

/**
 * A realm is authored as a versioned preset and *played* as a conversation.
 * The store spans both: `realm` is the open preset (editor mode); starting
 * a conversation instantiates it and moves the play state — cast snapshot,
 * events, turns — onto that conversation (playthrough mode).
 */
class RealmStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi(); // NOTE: This assumes it's accessed once during page init. Dirty workaround.
		return this._api;
	}

	summaries = $state<Array<RealmSummary>>([]);

	realm = $state<Realm | null>(null);

	// ----- playthrough state (bound to one conversation) -----

	conversation = $state<Conversation | null>(null);
	/** The cast pinned at instantiation — the playthrough's roster. */
	cast = $state<RealmCast | null>(null);
	/** Playthrough timeline, oldest-first, most recent window. */
	events = $state<Array<RealmEvent>>([]);
	/** Committed turns this session ≈ messages along the active path. */
	turnCount = $state(0);

	runStatus = $state<RunStatus>('idle');
	/** Keep running turns until a player's turn comes up (or an error/pause). */
	autoRun = $state(true);
	turn = $state<LiveTurn | null>(null);
	/** Set when the policy picked a player agent and waits on their turn. */
	awaitingAgentId = $state<Snowflake | null>(null);
	#cancelling = false;

	inspection = $state<AgentInspection | null>(null);

	/** Action space of the player agent whose turn it is. */
	playerActions = $state<ActionSpec[]>([]);

	get inPlaythrough(): boolean {
		return this.conversation !== null;
	}

	get isStreaming(): boolean {
		return this.runStatus === 'streaming';
	}

	/** The roster: pinned cast while playing, the live definition while editing. */
	get agents(): RealmAgent[] {
		return this.cast?.agents ?? this.realm?.definition.agents ?? [];
	}

	/** Whose turn the round-robin policy picks next (playthrough only). */
	get nextAgent(): RealmAgent | null {
		if (!this.inPlaythrough) return null;
		const agents = this.agents;
		if (agents.length === 0) return null;
		return agents[this.turnCount % agents.length];
	}

	get awaitingAgent(): RealmAgent | null {
		return this.agents.find((a) => a.id === this.awaitingAgentId) ?? null;
	}

	agentById(id: Snowflake | null): RealmAgent | undefined {
		return id ? this.agents.find((a) => a.id === id) : undefined;
	}

	// ------------------------------------------------------------- presets

	async loadList() {
		this.summaries = await this.#api.realms.list();
	}

	async createRealm(name: string): Promise<Realm> {
		const realm = await this.#api.realms.create({ name });
		await this.loadList();
		return realm;
	}

	async deleteRealm(id: Snowflake) {
		const prev = [...this.summaries];
		this.summaries = this.summaries.filter((r) => r.id !== id);
		if (this.realm?.id === id) this.closeRealm();
		try {
			await this.#api.realms.delete(id);
		} catch (e) {
			this.summaries = prev;
			this.#handleError(e, 'Failed to delete realm');
		}
	}

	async openRealm(id: Snowflake) {
		this.closeRealm();
		this.realm = await this.#api.realms.get(id);
	}

	closeRealm() {
		this.realm = null;
		this.inspection = null;
		this.#resetPlaythrough();
	}

	async refreshRealm() {
		if (!this.realm) return;
		this.realm = await this.#api.realms.get(this.realm.id);
	}

	async createAgent(req: CreateAgentRequest): Promise<RealmAgent | null> {
		if (!this.realm) return null;
		const agent = await this.#api.realms.agents.create(this.realm.id, req);
		this.realm.definition.agents = [...this.realm.definition.agents, agent];
		return agent;
	}

	async deleteAgent(agentId: Snowflake) {
		const realm = this.realm;
		if (!realm) return;
		const prev = [...realm.definition.agents];
		realm.definition.agents = realm.definition.agents.filter((a) => a.id !== agentId);
		if (this.inspection?.agentId === agentId) this.inspection = null;
		try {
			await this.#api.realms.agents.delete(realm.id, agentId);
			// turn_order was re-derived server-side; resync the document.
			await this.refreshRealm();
		} catch (e) {
			realm.definition.agents = prev;
			this.#handleError(e, 'Failed to delete agent');
		}
	}

	// -------------------------------------------------------- playthroughs

	/** Instantiate the open realm into a fresh conversation and enter play. */
	async startPlaythrough(): Promise<boolean> {
		const realm = this.realm;
		if (!realm || this.isStreaming) return false;

		try {
			const full = await this.#api.chat.realm.start({ realm_id: realm.id });
			const cast = await this.#api.chat.realm.cast(full.metadata.id);
			this.#resetPlaythrough();
			this.conversation = full.metadata;
			this.cast = cast;
			this.turnCount = full.messages.length;
			this.inspection = null;
			return true;
		} catch (e) {
			this.#handleError(e, 'Failed to start conversation');
			return false;
		}
	}

	/** Leave the playthrough and return to the preset editor. */
	exitPlaythrough() {
		this.#resetPlaythrough();
		this.inspection = null;
	}

	#resetPlaythrough() {
		this.conversation = null;
		this.cast = null;
		this.events = [];
		this.turnCount = 0;
		this.turn = null;
		this.awaitingAgentId = null;
		this.runStatus = 'idle';
		this.playerActions = [];
		this.#cancelling = false;
	}

	/** Load older events above the current window. Returns how many arrived. */
	async loadOlderEvents(): Promise<number> {
		const convo = this.conversation;
		const oldest = this.events[0];
		if (!convo || !oldest) return 0;
		const older = await this.#api.chat.realm.events(convo.id, oldest.id);
		if (older.length > 0) this.events = [...older, ...this.events];
		return older.length;
	}

	/** Run one turn. Returns true if it committed (so a loop may continue). */
	async step(): Promise<boolean> {
		const convo = this.conversation;
		if (!convo || this.isStreaming) return false;

		this.awaitingAgentId = null;
		this.runStatus = 'streaming';
		this.#cancelling = false;

		try {
			let committed = false;
			const stream = this.#api.chat.realm.turnStream(convo.id);
			for await (const event of stream) {
				switch (event.type) {
					case 'awaiting_player':
						this.awaitingAgentId = event.agent_id;
						break;
					case 'turn_started':
						this.turn = {
							agentId: event.agent_id,
							name: event.name,
							text: '',
							reasoning: '',
							widgets: [],
							actions: []
						};
						break;
					case 'delta':
						if (this.turn) this.turn.text += event.content;
						break;
					case 'reasoning':
						if (this.turn) this.turn.reasoning += event.content;
						break;
					case 'widget':
						if (this.turn) this.turn.widgets = [...this.turn.widgets, event.widget];
						break;
					case 'action':
						if (this.turn)
							this.turn.actions = [
								...this.turn.actions,
								{ name: event.name, is_error: event.is_error }
							];
						break;
					case 'committed':
						this.#applyCommit({ message: event.message, events: event.events });
						committed = true;
						break;
				}
			}
			this.turn = null;
			this.runStatus = 'idle';
			return committed;
		} catch (e) {
			this.turn = null;
			this.runStatus = 'error';
			if (!this.#cancelling) this.#handleError(e, 'Turn failed');
			else this.runStatus = 'idle';
			return false;
		} finally {
			this.#cancelling = false;
		}
	}

	/** Keep running turns until a player's turn, a pause, or an error. */
	async play() {
		this.autoRun = true;
		while (this.autoRun && this.conversation) {
			const committed = await this.step();
			if (!committed) break;
		}
	}

	pause() {
		this.autoRun = false;
	}

	/** Stop the in-flight generation; the abandoned turn never commits. */
	async cancel() {
		const convo = this.conversation;
		if (!convo || !this.isStreaming) return;
		this.autoRun = false;
		this.#cancelling = true;
		try {
			await this.#api.chat.cancel(convo.id);
		} catch (e) {
			this.#cancelling = false;
			console.warn('Failed to cancel turn', e);
		}
	}

	/** Say something, act, or both — then keep the loop going if auto-run is on. */
	async submitPlayerTurn(content: string | null, action: PlayerAction | null): Promise<boolean> {
		const convo = this.conversation;
		if (!convo || this.isStreaming) return false;

		let result: TurnResult;
		try {
			result = await this.#api.chat.realm.playerTurn(convo.id, { content, action });
		} catch (e) {
			this.#handleError(e, 'Failed to submit turn');
			return false;
		}

		this.awaitingAgentId = null;
		this.#applyCommit(result);
		if (this.autoRun) void this.play();
		return true;
	}

	async loadPlayerActions() {
		const convo = this.conversation;
		const agentId = this.awaitingAgentId;
		if (!convo || !agentId) {
			this.playerActions = [];
			return;
		}
		try {
			this.playerActions = await this.#api.chat.realm.agents.actions(convo.id, agentId);
		} catch (e) {
			this.playerActions = [];
			console.warn('Failed to load player actions', e);
		}
	}

	// ---------------------------------------------------------- inspection

	async inspectAgent(id: Snowflake) {
		if (this.inspection?.agentId === id) {
			this.inspection = null; // toggle off
			return;
		}
		const agent = this.agentById(id);
		if (!agent) return;

		try {
			if (this.conversation) {
				// Live playthrough: knowledge comes from the conversation.
				const api = this.#api.chat.realm.agents;
				const [actions, state, observations] = await Promise.all([
					api.actions(this.conversation.id, id),
					api.state(this.conversation.id, id),
					api.observations(this.conversation.id, id)
				]);
				this.inspection = { agentId: id, grants: agent.grants, actions, state, observations };
			} else if (this.realm) {
				// Editor: only the authored shape exists — no world to know yet.
				const actions = await this.#api.realms.agents.actions(this.realm.id, id);
				this.inspection = { agentId: id, grants: agent.grants, actions, state: {}, observations: [] };
			}
		} catch (e) {
			this.#handleError(e, 'Failed to inspect agent');
		}
	}

	async refreshInspection() {
		const id = this.inspection?.agentId;
		if (!id) return;
		this.inspection = null;
		await this.inspectAgent(id);
	}

	#applyCommit(result: TurnResult) {
		this.turnCount += 1;
		if (result.events.length > 0) this.events = [...this.events, ...result.events];
	}

	#handleError(e: unknown, title: string) {
		const message =
			e instanceof Error
				? e.message
				: e && typeof e === 'object' && 'message' in e
					? String((e as { message: unknown }).message)
					: String(e);
		showError(message, { title, durationMs: 5000 });
	}
}

export const realmStore = new RealmStore();
