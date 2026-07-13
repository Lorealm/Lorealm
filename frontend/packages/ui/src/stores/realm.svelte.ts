import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	ActionSpec,
	AgentGrant,
	AgentObservation,
	CreateAgentRequest,
	PlayerAction,
	Realm,
	RealmAgent,
	RealmEvent,
	RealmStreamEvent,
	RealmSummary,
	StateSlice,
	TickResponse
} from '$lib/types/realm.js';
import type { WidgetInstance } from '$lib/types/widgets.js';
import { showError } from './errorToast.svelte.ts';

export type RunStatus = 'idle' | 'streaming' | 'error';

/** The in-flight LLM turn, accumulated from the tick stream. */
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
	state: StateSlice;
	observations: AgentObservation[];
}

class RealmStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi(); // NOTE: This assumes it's accessed once during page init. Dirty workaround.
		return this._api;
	}

	summaries = $state<Array<RealmSummary>>([]);

	realm = $state<Realm | null>(null);
	agents = $state<Array<RealmAgent>>([]);
	/** Owner timeline, oldest-first, most recent window. */
	events = $state<Array<RealmEvent>>([]);

	runStatus = $state<RunStatus>('idle');
	/** Keep ticking until a player's turn comes up (or an error/pause). */
	autoRun = $state(true);
	turn = $state<LiveTurn | null>(null);
	/** Set when the policy picked a player agent and waits on their turn. */
	awaitingAgentId = $state<Snowflake | null>(null);
	#cancelling = false;

	inspection = $state<AgentInspection | null>(null);

	/** Action space of the player agent whose turn it is. */
	playerActions = $state<ActionSpec[]>([]);

	get isStreaming(): boolean {
		return this.runStatus === 'streaming';
	}

	get awaitingAgent(): RealmAgent | null {
		return this.agents.find((a) => a.id === this.awaitingAgentId) ?? null;
	}

	agentById(id: Snowflake | null): RealmAgent | undefined {
		return id ? this.agents.find((a) => a.id === id) : undefined;
	}

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
		const [realm, agents, events] = await Promise.all([
			this.#api.realms.get(id),
			this.#api.realms.agents.list(id),
			this.#api.realms.events(id)
		]);
		this.realm = realm;
		this.agents = agents;
		this.events = events;
	}

	closeRealm() {
		this.realm = null;
		this.agents = [];
		this.events = [];
		this.turn = null;
		this.awaitingAgentId = null;
		this.runStatus = 'idle';
		this.inspection = null;
		this.playerActions = [];
		this.#cancelling = false;
	}

	async refreshAgents() {
		if (!this.realm) return;
		this.agents = await this.#api.realms.agents.list(this.realm.id);
	}

	async createAgent(req: CreateAgentRequest): Promise<RealmAgent | null> {
		if (!this.realm) return null;
		const agent = await this.#api.realms.agents.create(this.realm.id, req);
		this.agents = [...this.agents, agent];
		return agent;
	}

	/** Load older events above the current window. Returns how many arrived. */
	async loadOlderEvents(): Promise<number> {
		const realm = this.realm;
		const oldest = this.events[0];
		if (!realm || !oldest) return 0;
		const older = await this.#api.realms.events(realm.id, oldest.tick);
		if (older.length > 0) this.events = [...older, ...this.events];
		return older.length;
	}

	/** Run one tick. Returns true if it committed (so a loop may continue). */
	async step(): Promise<boolean> {
		const realm = this.realm;
		if (!realm || this.isStreaming) return false;

		this.awaitingAgentId = null;
		this.runStatus = 'streaming';
		this.#cancelling = false;

		try {
			let committed = false;
			const stream = this.#api.realms.tickStream(realm.id);
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
						this.#applyCommit(event.tick, event.events);
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

	/** Keep ticking until a player's turn, a pause, or an error. */
	async play() {
		this.autoRun = true;
		while (this.autoRun && this.realm) {
			const committed = await this.step();
			if (!committed) break;
		}
	}

	pause() {
		this.autoRun = false;
	}

	/** Stop the in-flight generation; the abandoned turn never commits. */
	async cancel() {
		const realm = this.realm;
		if (!realm || !this.isStreaming) return;
		this.autoRun = false;
		this.#cancelling = true;
		try {
			await this.#api.realms.cancelTick(realm.id);
		} catch (e) {
			this.#cancelling = false;
			console.warn('Failed to cancel tick', e);
		}
	}

	/** Say something, act, or both — then keep the loop going if auto-run is on. */
	async submitPlayerTurn(content: string | null, action: PlayerAction | null): Promise<boolean> {
		const realm = this.realm;
		if (!realm || this.isStreaming) return false;

		let result: TickResponse;
		try {
			result = await this.#api.realms.playerTurn(realm.id, {
				content,
				action
			});
		} catch (e) {
			this.#handleError(e, 'Failed to submit turn');
			return false;
		}

		if (result.status === 'committed') {
			this.awaitingAgentId = null;
			this.#applyCommit(result.tick, result.events);
			if (this.autoRun) void this.play();
			return true;
		}
		// awaiting_player: someone else's turn — surface whose.
		this.awaitingAgentId = result.agent_id;
		return false;
	}

	async loadPlayerActions() {
		const realm = this.realm;
		const agentId = this.awaitingAgentId;
		if (!realm || !agentId) {
			this.playerActions = [];
			return;
		}
		try {
			this.playerActions = await this.#api.realms.agents.actions(realm.id, agentId);
		} catch (e) {
			this.playerActions = [];
			console.warn('Failed to load player actions', e);
		}
	}

	async inspectAgent(id: Snowflake) {
		const realm = this.realm;
		if (!realm) return;
		if (this.inspection?.agentId === id) {
			this.inspection = null; // toggle off
			return;
		}
		const api = this.#api.realms.agents;
		try {
			const [grants, actions, state, observations] = await Promise.all([
				api.grants.get(realm.id, id),
				api.actions(realm.id, id),
				api.state(realm.id, id),
				api.observations(realm.id, id)
			]);
			this.inspection = { agentId: id, grants, actions, state, observations };
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

	#applyCommit(tick: number, events: RealmEvent[]) {
		// `tick` is the tick that just committed; the realm now sits past it.
		if (this.realm) this.realm.tick = tick + 1;
		if (events.length > 0) this.events = [...this.events, ...events];
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
