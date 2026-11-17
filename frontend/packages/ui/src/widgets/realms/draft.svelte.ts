import type { Snowflake } from '#lib/types/core.js';
import type { AgentKind, Director, RealmResourceKind } from '#lib/types/realm.js';
import type {
	InputSpec,
	InputSpecType,
	Outline,
	OutlineAgent,
	OutlineBinding
} from '#lib/types/blueprint.js';
import {
	loopReaches,
	parseGrant,
	emptyAgent,
	emptyScenario,
	freshKey,
	declaredText,
	selectionId,
	type AgentBlock,
	type Binding,
	type CastEntry,
	type InputDecl,
	type InputType,
	type ScenarioBlock,
	type Selection
} from './model.js';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * A declaration as the server reports it, back in the editor's shape.
 */
function fromSpec(spec: InputSpec): InputDecl {
	const ty = (from: InputSpec | InputSpecType): InputType => {
		switch (from.type) {
			case 'enum':
				return { type: 'enum', variants: from.variants };
			case 'range':
				return { type: 'range', min: from.min, max: from.max };
			case 'span':
				return { type: 'span', min: from.min, max: from.max };
			case 'list':
				return { type: 'list', of: ty(from.of) };
			case 'object':
				return {
					type: 'object',
					fields: from.fields.map((f) => ({ name: f.name, ty: ty(f) }))
				};
			case 'reference':
				return { type: 'reference', kind: from.kind };
			default:
				return { type: from.type };
		}
	};
	return {
		name: spec.name,
		ty: ty(spec),
		defaultValue: spec.required ? null : '',
		doc: spec.doc ?? ''
	};
}

/** Every agent slot that holds a single binding. Lorebooks are a list apart. */
const BINDING_SLOTS = ['character', 'template', 'persona', 'profile'] as const;

/**
 * The document the editor is currently working on, held entirely in memory.
 */
class RealmDraft {
	name = $state('');
	description = $state('');
	/** Markdown shown before the inputs when a realm is built. */
	introduction = $state('');
	tags = $state<string[]>([]);
	/** Null = RoundRobin. */
	director = $state<Director | null>(null);
	config = $state<Record<string, string>>({});
	scenarios = $state<ScenarioBlock[]>([]);
	inputs = $state<InputDecl[]>([]);
	cast = $state<CastEntry[]>([]);
	modules = $state<Binding[]>([]);
	selection = $state<Selection>(null);
	fieldPing = $state<{ name: string; path: string[] } | null>(null);

	get isEmpty(): boolean {
		return this.cast.length === 0 && this.inputs.length === 0 && this.scenarios.length === 0;
	}

	/** True once anything references an input: the document is a blueprint. */
	get isBlueprint(): boolean {
		return this.inputs.length > 0;
	}

	/**
	 * Load a document's structure, as the server read it back.
	 */
	load(outline: Outline, nameOf: (kind: RealmResourceKind, id: Snowflake) => string) {
		const bind = (binding: OutlineBinding | null): Binding | null => {
			if (!binding) return null;
			if (binding.source === 'resource') {
				return {
					source: 'resource',
					kind: binding.kind,
					id: binding.id,
					name: nameOf(binding.kind, binding.id)
				};
			}
			return binding.source === 'loop'
				? { source: 'loop', varName: binding.varName, path: binding.path }
				: { source: 'input', name: binding.name, path: binding.path };
		};

		const agent = (from: OutlineAgent): AgentBlock => ({
			key: freshKey('agent-'),
			handle: from.handle,
			name: from.name,
			kind: (['llm', 'player', 'script'] as const).includes(from.kind as AgentKind)
				? (from.kind as AgentKind)
				: 'llm',
			character: bind(from.character),
			template: bind(from.template),
			persona: bind(from.persona),
			profile: bind(from.profile),
			lorebooks: from.lorebooks.map(bind).filter((b): b is Binding => b !== null),
			widgets: from.widgets.map(bind).filter((b): b is Binding => b !== null),
			tools: from.tools.flatMap((t) => {
				const tool = bind(t.tool);
				return tool ? [{ tool, eventChannel: t.event_channel }] : [];
			}),
			grants: from.grants.map((g) => parseGrant(g.channel, g.access)),
			properties: from.properties.map((p) => ({ ...p })),
			speechChannel: from.speech_channel,
			script: from.script,
			run_condition: from.run_condition
		});

		this.name = outline.name;
		this.description = outline.description;
		this.introduction = outline.introduction;
		this.tags = outline.tags;
		this.director = outline.director;
		this.config = { ...outline.config };
		this.scenarios = outline.scenarios.map((s) => ({ ...s, key: freshKey('scenario-') }));
		this.inputs = outline.inputs.map(fromSpec);
		this.cast = outline.cast.map((entry) =>
			entry.kind === 'agent'
				? { key: freshKey('entry-'), kind: 'agent' as const, agent: agent(entry.agent) }
				: {
						key: freshKey('entry-'),
						kind: 'foreach' as const,
						varName: entry.var_name,
						over: entry.over,
						body: entry.body.map(agent)
					}
		);
		this.modules = outline.modules.map(bind).filter((b): b is Binding => b !== null);
		this.selection = null;
	}

	reset() {
		this.name = '';
		this.description = '';
		this.introduction = '';
		this.tags = [];
		this.director = null;
		this.config = {};
		this.scenarios = [];
		this.inputs = [];
		this.cast = [];
		this.modules = [];
		this.selection = null;
	}

	select(selection: Selection) {
		this.selection = selection;
		this.fieldPing = null;
	}

	pingField(name: string, path: string[]) {
		this.selection = { kind: 'input', name };
		this.fieldPing = { name, path };
	}

	isSelected(selection: NonNullable<Selection>): boolean {
		return this.selection !== null && selectionId(this.selection) === selectionId(selection);
	}

	agent(key: string): AgentBlock | null {
		for (const entry of this.cast) {
			if (entry.kind === 'agent') {
				if (entry.agent.key === key) return entry.agent;
			} else {
				const found = entry.body.find((a) => a.key === key);
				if (found) return found;
			}
		}
		return null;
	}

	/** The loop variables an agent can read */
	scopeOf(agentKey: string): Array<{ varName: string; input: InputDecl | null }> {
		for (const entry of this.cast) {
			if (entry.kind !== 'foreach') continue;
			if (!entry.body.some((a) => a.key === agentKey)) continue;
			const input = this.inputs.find((i) => i.name === entry.over) ?? null;
			// A loop is only useful here if its variable reaches a reference
			if (!input || loopReaches(input.ty).length === 0) return [];
			return [{ varName: entry.varName, input }];
		}
		return [];
	}

	addAgent(): AgentBlock {
		const agent = emptyAgent(this.#uniqueHandle('agent'));
		this.cast.push({ key: freshKey('entry-'), kind: 'agent', agent });
		this.select({ kind: 'agent', key: agent.key });
		return agent;
	}

	addForeach(): CastEntry {
		const entry: CastEntry = {
			key: freshKey('entry-'),
			kind: 'foreach',
			varName: 'item',
			over: null,
			body: [emptyAgent(this.#uniqueHandle('member'))]
		};
		this.cast.push(entry);
		this.select({ kind: 'foreach', key: entry.key });
		return entry;
	}

	/** Append an agent to a loop body. */
	addToBody(entryKey: string) {
		const entry = this.cast.find((e) => e.key === entryKey);
		if (entry?.kind !== 'foreach') return;
		const agent = emptyAgent(this.#uniqueHandle('member'));
		entry.body.push(agent);
		this.select({ kind: 'agent', key: agent.key });
	}

	removeEntry(key: string) {
		this.cast = this.cast.filter((e) => e.key !== key);
		this.selection = null;
	}

	removeAgent(key: string) {
		for (const entry of this.cast) {
			if (entry.kind !== 'foreach') continue;
			const at = entry.body.findIndex((a) => a.key === key);
			if (at >= 0) {
				entry.body.splice(at, 1);
				this.selection = null;
				return;
			}
		}
		const at = this.cast.findIndex((e) => e.kind === 'agent' && e.agent.key === key);
		if (at >= 0) {
			this.cast.splice(at, 1);
			this.selection = null;
		}
	}

	/** Move a top-level entry so it lands at `to` */
	moveEntry(key: string, to: number) {
		const from = this.cast.findIndex((e) => e.key === key);
		if (from < 0) return;
		const target = Math.max(0, Math.min(to > from ? to - 1 : to, this.cast.length - 1));
		if (target === from) return;
		const [entry] = this.cast.splice(from, 1);
		this.cast.splice(target, 0, entry);
	}

	moveAgent(agentKey: string, parent: string | null, index: number) {
		const into = parent === null ? null : this.cast.find((e) => e.key === parent);
		if (parent !== null && into?.kind !== 'foreach') return;

		let agent: AgentBlock | undefined;
		let fromParent: string | null = null;
		let fromIndex = -1;
		for (const entry of this.cast) {
			if (entry.kind !== 'foreach') continue;
			const at = entry.body.findIndex((a) => a.key === agentKey);
			if (at < 0) continue;
			[agent] = entry.body.splice(at, 1);
			fromParent = entry.key;
			fromIndex = at;
			break;
		}
		if (!agent) {
			const at = this.cast.findIndex((e) => e.kind === 'agent' && e.agent.key === agentKey);
			if (at < 0) return;
			const [entry] = this.cast.splice(at, 1);
			agent = (entry as { kind: 'agent'; agent: AgentBlock }).agent;
			fromIndex = at;
		}

		if (fromParent !== parent) {
			for (const slot of BINDING_SLOTS) {
				if (agent[slot]?.source === 'loop') agent[slot] = null;
			}
			agent.lorebooks = agent.lorebooks.filter((b) => b.source !== 'loop');
		}

		const to = fromParent === parent && index > fromIndex ? index - 1 : index;
		if (into?.kind === 'foreach') {
			into.body.splice(Math.max(0, Math.min(to, into.body.length)), 0, agent);
		} else {
			this.cast.splice(Math.max(0, Math.min(to, this.cast.length)), 0, {
				key: freshKey('entry-'),
				kind: 'agent',
				agent
			});
		}
	}

	addScenario(): ScenarioBlock {
		const scenario = emptyScenario();
		scenario.name = t.realms.scenarios.defaultName({ n: this.scenarios.length + 1 });
		this.scenarios.push(scenario);
		return scenario;
	}

	removeScenario(key: string) {
		this.scenarios = this.scenarios.filter((s) => s.key !== key);
	}

	addInput(): InputDecl {
		const decl: InputDecl = {
			name: this.#uniqueInputName('input'),
			ty: { type: 'list', of: { type: 'reference', kind: 'character' } },
			defaultValue: null,
			doc: ''
		};
		this.inputs.push(decl);
		this.select({ kind: 'input', name: decl.name });
		return decl;
	}

	/** Rename a declaration */
	renameInput(from: string, to: string) {
		const decl = this.inputs.find((i) => i.name === from);
		if (!decl || from === to) return;
		const name = this.#uniqueInputName(to || 'input', from);
		decl.name = name;
		for (const entry of this.cast) {
			if (entry.kind === 'foreach' && entry.over === from) entry.over = name;
		}
		this.#rewriteInputBindings(from, name);
		if (this.selection?.kind === 'input' && this.selection.name === from) {
			this.selection = { kind: 'input', name };
		}
	}

	removeInput(name: string) {
		this.inputs = this.inputs.filter((i) => i.name !== name);
		for (const entry of this.cast) {
			if (entry.kind === 'foreach' && entry.over === name) entry.over = null;
		}
		this.#rewriteInputBindings(name, null);
		this.selection = null;
	}

	/**
	 * Retype a declaration, dropping the fields that read it if it can no longer
	 * fill them.
	 */
	retypeInput(name: string, ty: InputType) {
		const decl = this.inputs.find((i) => i.name === name);
		if (!decl) return;
		const before = declaredText(decl.ty);
		decl.ty = ty;
		if (before !== declaredText(ty)) this.#rewriteInputBindings(name, null);
	}

	#rewriteInputBindings(from: string, to: string | null) {
		const reads = (binding: Binding) => binding.source === 'input' && binding.name === from;
		const moved = (binding: Binding): Binding =>
			binding.source === 'input' && to !== null ? { ...binding, name: to } : binding;

		for (const agent of this.#agents()) {
			for (const slot of BINDING_SLOTS) {
				const binding = agent[slot];
				if (binding && reads(binding)) {
					agent[slot] = to === null ? null : moved(binding);
				}
			}
			agent.lorebooks =
				to === null
					? agent.lorebooks.filter((b) => !reads(b))
					: agent.lorebooks.map((b) => (reads(b) ? moved(b) : b));
		}
	}

	#agents(): AgentBlock[] {
		return this.cast.flatMap((entry) => (entry.kind === 'agent' ? [entry.agent] : entry.body));
	}

	/** Drop every binding that reads a loop variable no longer in scope. */
	clearStaleBindings(entryKey: string) {
		const entry = this.cast.find((e) => e.key === entryKey);
		if (entry?.kind !== 'foreach') return;
		for (const agent of entry.body) {
			for (const field of ['character', 'template', 'persona', 'profile'] as const) {
				if (agent[field]?.source === 'loop') agent[field] = null;
			}
			agent.lorebooks = agent.lorebooks.filter((b: Binding) => b.source !== 'loop');
		}
	}

	#handles(): string[] {
		return this.#agents().map((a) => a.handle);
	}

	#uniqueHandle(base: string): string {
		const taken = this.#handles();
		let n = 1;
		while (taken.includes(`${base}_${n}`)) n++;
		return `${base}_${n}`;
	}

	#uniqueInputName(base: string, ignore?: string): string {
		const taken = this.inputs.filter((i) => i.name !== ignore).map((i) => i.name);
		if (!taken.includes(base)) return base;
		let n = 2;
		while (taken.includes(`${base}${n}`)) n++;
		return `${base}${n}`;
	}
}

export const realmDraft = new RealmDraft();
