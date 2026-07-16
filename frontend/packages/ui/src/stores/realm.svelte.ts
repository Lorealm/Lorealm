import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Conversation } from '$lib/types/conversation.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	ActionSpec,
	AgentGrant,
	AgentObservation,
	CreateAgentRequest,
	ImportKind,
	ImportRule,
	PlayerAction,
	Realm,
	RealmAgent,
	RealmCast,
	RealmConfig,
	RealmEvent,
	RealmSummary,
	RealmVersionInfo,
	StateSlice,
	TurnResult,
	UpdateRealmRequest
} from '$lib/types/realm.js';
import type { WidgetInstance } from '$lib/types/widgets.js';
import { SvelteSet } from 'svelte/reactivity';
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
	state: StateSlice;
	/** The agent's private feed. */
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

	conversation = $state<Conversation | null>(null);
	cast = $state<RealmCast | null>(null);
	events = $state<Array<RealmEvent>>([]);
	turnCount = $state(0);

	runStatus = $state<RunStatus>('idle');
	autoRun = $state(true);
	turn = $state<LiveTurn | null>(null);
	awaitingAgentId = $state<Snowflake | null>(null);
	#cancelling = false;

	inspection = $state<AgentInspection | null>(null);

	playerActions = $state<ActionSpec[]>([]);

	get inPlaythrough(): boolean {
		return this.conversation !== null;
	}

	get isStreaming(): boolean {
		return this.runStatus === 'streaming';
	}

	get agents(): RealmAgent[] {
		return this.cast?.agents ?? this.realm?.definition.agents ?? [];
	}

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
		await this.loadConfig();
	}

	closeRealm() {
		this.realm = null;
		this.inspection = null;
		this.versions = [];
		this.config = {};
		this.#resetPlaythrough();
	}

	async refreshRealm() {
		if (!this.realm) return;
		this.realm = await this.#api.realms.get(this.realm.id);
	}

	async updateRealm(req: UpdateRealmRequest): Promise<boolean> {
		if (!this.realm) return false;
		try {
			this.realm = await this.#api.realms.update(this.realm.id, req);
			await this.loadList();
			return true;
		} catch (e) {
			this.#handleError(e, 'Failed to update realm');
			return false;
		}
	}

	async setImports(imports: ImportRule[]): Promise<void> {
		const realm = this.realm;
		if (!realm) return;
		realm.definition.imports = await this.#api.realms.imports.set(realm.id, imports);
	}

	config = $state<RealmConfig>({});

	async loadConfig() {
		if (!this.realm) return;
		try {
			this.config = await this.#api.realms.config.get(this.realm.id);
		} catch (e) {
			this.#handleError(e, 'Failed to load formatting settings');
		}
	}

	async saveConfig(config: RealmConfig): Promise<boolean> {
		if (!this.realm) return false;
		try {
			this.config = await this.#api.realms.config.set(this.realm.id, config);
			return true;
		} catch (e) {
			this.#handleError(e, 'Failed to save formatting');
			return false;
		}
	}

	importUsers(kind: ImportKind, resourceId: Snowflake): RealmAgent[] {
		const agents = this.realm?.definition.agents ?? [];
		return agents.filter((a) => {
			switch (kind) {
				case 'character':
					return a.character_id === resourceId;
				case 'template':
					return a.template_id === resourceId;
				case 'tool':
					return a.tools.some((b) => b.tool_id === resourceId);
				case 'lorebook':
					return a.lorebooks.includes(resourceId);
			}
		});
	}

	get missingImports(): ImportRule[] {
		const definition = this.realm?.definition;
		if (!definition) return [];
		const have = new SvelteSet(definition.imports.map((r) => `${r.kind}:${r.resource_id}`));
		const missing: ImportRule[] = [];
		const need = (kind: ImportKind, resource_id: Snowflake) => {
			const key = `${kind}:${resource_id}`;
			if (have.has(key)) return;
			have.add(key);
			missing.push({ kind, resource_id });
		};
		for (const agent of definition.agents) {
			if (agent.character_id) need('character', agent.character_id);
			if (agent.template_id) need('template', agent.template_id);
			for (const b of agent.tools) need('tool', b.tool_id);
			for (const l of agent.lorebooks ?? []) need('lorebook', l);
		}
		return missing;
	}

	async createAgent(req: CreateAgentRequest): Promise<RealmAgent | null> {
		if (!this.realm) return null;
		const agent = await this.#api.realms.agents.create(this.realm.id, req);
		this.realm.definition.agents = [...this.realm.definition.agents, agent];
		return agent;
	}

	async reorderAgents(order: Snowflake[]) {
		const realm = this.realm;
		if (!realm) return;
		const prev = [...realm.definition.agents];
		const byId = new Map(prev.map((a) => [a.id, a]));
		realm.definition.agents = order
			.map((id) => byId.get(id))
			.filter((a): a is RealmAgent => a !== undefined);
		try {
			realm.definition.agents = await this.#api.realms.agents.reorder(realm.id, order);
		} catch (e) {
			realm.definition.agents = prev;
			this.#handleError(e, 'Failed to reorder cast');
		}
	}

	async moveAgent(agentId: Snowflake, delta: -1 | 1) {
		const agents = this.realm?.definition.agents ?? [];
		const index = agents.findIndex((a) => a.id === agentId);
		const target = index + delta;
		if (index === -1 || target < 0 || target >= agents.length) return;
		const order = agents.map((a) => a.id);
		[order[index], order[target]] = [order[target], order[index]];
		await this.reorderAgents(order);
	}

	async updateAgent(agentId: Snowflake, req: CreateAgentRequest): Promise<RealmAgent | null> {
		const realm = this.realm;
		if (!realm) return null;
		const agent = await this.#api.realms.agents.update(realm.id, agentId, req);
		realm.definition.agents = realm.definition.agents.map((a) => (a.id === agentId ? agent : a));
		if (this.inspection?.agentId === agentId) await this.refreshInspection();
		return agent;
	}

	agentRequest(a: RealmAgent): CreateAgentRequest {
		return {
			name: a.name,
			kind: a.kind,
			character_id: a.character_id,
			char_version_id: a.char_version_id,
			template_id: a.template_id,
			profile_id: a.profile_id,
			persona_id: a.persona_id,
			grants: a.grants,
			tools: a.tools,
			speech_scope: a.speech_scope ?? null,
			lorebooks: a.lorebooks,
			script: a.script ?? null,
			run_condition: a.run_condition ?? null
		};
	}

	#agentSaveTimers = new Map<Snowflake, ReturnType<typeof setTimeout>>();

	editAgent(agentId: Snowflake, mutate: (a: RealmAgent) => void) {
		const agent = this.realm?.definition.agents.find((a) => a.id === agentId);
		if (!agent) return;
		mutate(agent);
		this.#queueAgentSave(agentId);
	}

	async setAgentCharacter(agentId: Snowflake, characterId: Snowflake | '') {
		const agent = this.realm?.definition.agents.find((a) => a.id === agentId);
		if (!agent) return;
		if (!characterId) {
			agent.character_id = null;
			agent.char_version_id = null;
		} else {
			try {
				const full = await this.#api.characters.get(characterId);
				agent.character_id = full.id;
				agent.char_version_id = full.current_version_id;
			} catch (e) {
				this.#handleError(e, 'Failed to load character');
				return;
			}
		}
		this.#queueAgentSave(agentId);
	}

	#queueAgentSave(agentId: Snowflake) {
		const existing = this.#agentSaveTimers.get(agentId);
		if (existing) clearTimeout(existing);
		this.#agentSaveTimers.set(
			agentId,
			setTimeout(() => void this.#saveAgentNow(agentId), 500)
		);
	}

	async #saveAgentNow(agentId: Snowflake) {
		this.#agentSaveTimers.delete(agentId);
		const realm = this.realm;
		const agent = realm?.definition.agents.find((a) => a.id === agentId);
		if (!realm || !agent) return;
		try {
			const saved = await this.#api.realms.agents.update(
				realm.id,
				agentId,
				this.agentRequest(agent)
			);
			agent.turn_order = saved.turn_order;
			agent.char_version_id = saved.char_version_id;
		} catch (e) {
			this.#handleError(e, 'Failed to save agent');
		}
	}

	/** Flush any pending debounced agent saves immediately. */
	async flushAgentSaves() {
		const ids = [...this.#agentSaveTimers.keys()];
		for (const id of ids) {
			clearTimeout(this.#agentSaveTimers.get(id));
			await this.#saveAgentNow(id);
		}
	}

	async deleteAgent(agentId: Snowflake) {
		const realm = this.realm;
		if (!realm) return;
		const timer = this.#agentSaveTimers.get(agentId);
		if (timer) {
			clearTimeout(timer);
			this.#agentSaveTimers.delete(agentId);
		}
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

	versions = $state<RealmVersionInfo[]>([]);

	async loadVersions() {
		if (!this.realm) return;
		try {
			this.versions = await this.#api.realms.versions.list(this.realm.id);
		} catch (e) {
			this.#handleError(e, 'Failed to load version history');
		}
	}

	async restoreVersion(versionId: Snowflake): Promise<boolean> {
		if (!this.realm) return false;
		try {
			this.realm = await this.#api.realms.versions.restore(this.realm.id, versionId);
			await this.loadVersions();
			return true;
		} catch (e) {
			this.#handleError(e, 'Failed to restore version');
			return false;
		}
	}

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

	async loadOlderEvents(): Promise<number> {
		const convo = this.conversation;
		const oldest = this.events[0];
		if (!convo || !oldest) return 0;
		const older = await this.#api.chat.realm.events(convo.id, oldest.id);
		if (older.length > 0) this.events = [...older, ...this.events];
		return older.length;
	}

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

	async inspectAgent(id: Snowflake) {
		if (this.inspection?.agentId === id) {
			this.inspection = null; // toggle off
			return;
		}
		await this.selectInspection(id);
	}

	async selectInspection(id: Snowflake) {
		if (this.inspection?.agentId === id) return;
		const agent = this.agentById(id);
		if (!agent) return;

		try {
			if (this.conversation) {
				// Live playthrough: state and observations come from the conversation.
				const api = this.#api.chat.realm.agents;
				const [actions, state, observations] = await Promise.all([
					api.actions(this.conversation.id, id),
					api.state(this.conversation.id, id),
					api.observations(this.conversation.id, id)
				]);
				this.inspection = { agentId: id, grants: agent.grants, actions, state, observations };
			} else if (this.realm) {
				// Editor: only the authored shape exists, no live state.
				const actions = await this.#api.realms.agents.actions(this.realm.id, id);
				this.inspection = {
					agentId: id,
					grants: agent.grants,
					actions,
					state: {},
					observations: []
				};
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
