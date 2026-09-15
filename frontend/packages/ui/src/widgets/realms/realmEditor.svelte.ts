import { characterStore } from '$lib/stores/character.svelte.js';
import { connectionStore } from '$lib/stores/connection.svelte.js';
import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
import { personaStore } from '$lib/stores/persona.svelte.js';
import { templateStore } from '$lib/stores/template.svelte.js';
import { toolStore } from '$lib/stores/tools.svelte.js';
import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
import type { Snowflake } from '$lib/types/core.js';
import type { Realm, RealmDefinition, RealmResourceKind, RealmResourceRef } from '$lib/types/realm.js';

export type EditorTab =
	| 'overview'
	| 'cast'
	| 'scenarios'
	| 'access'
	| 'graph'
	| 'formatting'
	| 'history';

export const EDITOR_TABS: Array<{ value: EditorTab; label: string }> = [
	{ value: 'overview', label: 'Overview' },
	{ value: 'cast', label: 'Cast' },
	{ value: 'scenarios', label: 'Scenarios' },
	{ value: 'access', label: 'Access Map' },
	{ value: 'graph', label: 'Graph (Beta)' },
	{ value: 'formatting', label: 'Formatting' },
	{ value: 'history', label: 'History' }
];

/** A catalog resource projected to the shape the editor renders. */
export interface CatalogItem {
	id: Snowflake;
	name: string;
	description: string;
}

/** The owner's resources of one kind, read from the catalog stores. */
export function catalogItems(kind: RealmResourceKind): CatalogItem[] {
	switch (kind) {
		case 'character':
			return (characterStore.metadataList ?? []).map((c) => ({
				id: c.id,
				name: c.display_name ?? c.name,
				description: ''
			}));
		case 'lorebook':
			return (lorebookStore.metadataList ?? []).map((l) => ({
				id: l.id,
				name: l.name,
				description: l.description
			}));
		case 'template':
			return (templateStore.metadataList ?? []).map((t) => ({
				id: t.id,
				name: t.name,
				description: t.description
			}));
		case 'tool':
			return (toolStore.loadedTools ?? []).map((t) => ({
				id: t.id,
				name: t.display_name ?? t.name,
				description: t.description ?? ''
			}));
		case 'persona':
			return (personaStore.loadedPersonas ?? []).map((p) => ({
				id: p.id,
				name: p.display_name ?? p.name,
				description: p.description ?? ''
			}));
		case 'connection_profile':
			return (connectionStore.profiles ?? []).map((p) => ({
				id: p.id,
				name: p.name,
				description: p.provider
			}));
		case 'widget':
			return (userWidgetStore.loadedWidgets ?? []).map((w) => ({
				id: w.id,
				name: w.name,
				description: w.description
			}));
	}
}

export function catalogItem(kind: RealmResourceKind, id: Snowflake): CatalogItem | undefined {
	return catalogItems(kind).find((c) => c.id === id);
}

/** Server-shipped builtin tools: bindable without an import. */
export function builtinToolItems(): CatalogItem[] {
	return (toolStore.builtinTools ?? []).map((t) => ({
		id: t.id,
		name: t.display_name ?? t.name,
		description: t.description ?? ''
	}));
}

/**
 * Every resource the cast references, deduplicated, in cast order.
 */
export function realmResources(definition: RealmDefinition): RealmResourceRef[] {
	const seen = new Set<string>();
	const out: RealmResourceRef[] = [];
	const push = (kind: RealmResourceKind, resource_id: Snowflake) => {
		const key = `${kind}:${resource_id}`;
		if (seen.has(key)) return;
		seen.add(key);
		out.push({ kind, resource_id });
	};

	for (const a of definition.agents) {
		if (a.character_id) push('character', a.character_id);
		if (a.template_id) push('template', a.template_id);
		if (a.profile_id) push('connection_profile', a.profile_id);
		if (a.persona_id) push('persona', a.persona_id);
		for (const b of a.tools) push('tool', b.tool_id);
		for (const l of a.lorebooks ?? []) push('lorebook', l);
		for (const w of a.widgets ?? []) push('widget', w);
	}
	return out;
}

export const scopeRoot = (scope: string): string => scope.split(':')[0];

export const GRAPH_W = 900;
export const GRAPH_H = 620;
export const POOL_W = 190;
export const POOL_HEADER_H = 26;
export const SCOPE_ROW_H = 24;
export const POOL_PAD = 8;

export type GraphType = 'agent' | 'scope' | 'tool' | 'lorebook' | 'character' | 'template';

interface LayoutNode {
	id: string;
	type: GraphType | 'pool';
	mass: number;
	label: string;
	unused: boolean;
	x: number;
	y: number;
	vx: number;
	vy: number;
	halfW: number;
	halfH: number;
}

interface LayoutEdge {
	source: string;
	target: string;
}

/** Force-directed placement */
function relax(nodes: LayoutNode[], edges: LayoutEdge[], W: number, H: number) {
	const byId: Record<string, LayoutNode> = {};
	const place = (list: LayoutNode[], cx: number) => {
		const m = list.length;
		list.forEach((n, i) => {
			n.x = cx + (i % 2 ? 26 : -26);
			n.y = (H * (i + 1)) / (m + 1);
			n.vx = 0;
			n.vy = 0;
			byId[n.id] = n;
		});
	};
	place(
		nodes.filter((n) => n.type === 'pool'),
		W * 0.24
	);
	place(
		nodes.filter((n) => n.type === 'agent'),
		W * 0.5
	);
	place(
		nodes.filter((n) => n.type !== 'pool' && n.type !== 'agent'),
		W * 0.78
	);
	for (let iter = 0; iter < 320; iter++) {
		for (let i = 0; i < nodes.length; i++) {
			for (let j = i + 1; j < nodes.length; j++) {
				const a = nodes[i];
				const b = nodes[j];
				const dx = a.x - b.x;
				const dy = a.y - b.y;
				let d2 = dx * dx + dy * dy;
				if (d2 < 1) d2 = 1;
				const d = Math.sqrt(d2);
				const force = (2600 * a.mass * b.mass) / d2;
				const fx = (dx / d) * force;
				const fy = (dy / d) * force;
				a.vx += fx;
				a.vy += fy;
				b.vx -= fx;
				b.vy -= fy;
			}
		}
		edges.forEach((e) => {
			const a = byId[e.source];
			const b = byId[e.target];
			if (!a || !b) return;
			const rest = e.target.split(':')[0] === 'pool' ? 175 : 120;
			const dx = b.x - a.x;
			const dy = b.y - a.y;
			const d = Math.sqrt(dx * dx + dy * dy) || 0.01;
			const force = (d - rest) * 0.02;
			const fx = (dx / d) * force;
			const fy = (dy / d) * force;
			a.vx += fx;
			a.vy += fy;
			b.vx -= fx;
			b.vy -= fy;
		});
		nodes.forEach((n) => {
			n.vx += (W / 2 - n.x) * 0.0008;
			n.vy += (H / 2 - n.y) * 0.0008;
			n.vx *= 0.85;
			n.vy *= 0.85;
			n.x += n.vx;
			n.y += n.vy;
			n.x = Math.max(n.halfW + 8, Math.min(W - n.halfW - 8, n.x));
			n.y = Math.max(n.halfH + 8, Math.min(H - n.halfH - 8, n.y));
		});
	}
}

export interface GraphModel {
	/** Everything except pools; pools render as their own boxes. */
	nodes: Array<{ id: string; type: GraphType; label: string; unused: boolean }>;
	poolRoots: string[];
	poolScopes: Record<string, string[]>;
	scopeList: string[];
	positions: Record<string, { x: number; y: number }>;
}

let graphCache: { sig: string; model: GraphModel } = {
	sig: '',
	model: { nodes: [], poolRoots: [], poolScopes: {}, scopeList: [], positions: {} }
};

/** Build (and cache) the realm's resource graph and its node positions */
export function buildGraph(realm: Realm): GraphModel {
	const agents = realm.definition.agents;
	const resources = realmResources(realm.definition);
	const referenced = (kind: RealmResourceKind): CatalogItem[] =>
		resources
			.filter((r) => r.kind === kind)
			.map(
				({ resource_id: id }) =>
					catalogItem(kind, id) ?? { id, name: `#${id}`, description: '' }
			);
	const tools = referenced('tool');
	const lorebooks = referenced('lorebook');
	const characters = referenced('character');
	const templates = referenced('template');

	const scopeList: string[] = [];
	agents.forEach((a) =>
		a.grants.forEach((g) => {
			if (g.channel && !scopeList.includes(g.channel)) scopeList.push(g.channel);
		})
	);
	scopeList.sort();

	const poolRoots: string[] = [];
	const poolScopes: Record<string, string[]> = {};
	scopeList.forEach((sc) => {
		const r = scopeRoot(sc);
		if (!poolScopes[r]) {
			poolScopes[r] = [];
			poolRoots.push(r);
		}
		poolScopes[r].push(sc);
	});

	const nodes: GraphModel['nodes'] = [];
	agents.forEach((a) =>
		nodes.push({ id: 'agent:' + a.id, type: 'agent', label: a.name, unused: false })
	);
	tools.forEach((t) =>
		nodes.push({
			id: 'tool:' + t.id,
			type: 'tool',
			label: t.name,
			unused: false
		})
	);
	lorebooks.forEach((lb) =>
		nodes.push({
			id: 'lorebook:' + lb.id,
			type: 'lorebook',
			label: lb.name,
			unused: false
		})
	);
	characters.forEach((c) =>
		nodes.push({
			id: 'character:' + c.id,
			type: 'character',
			label: c.name,
			unused: false
		})
	);
	templates.forEach((t) =>
		nodes.push({
			id: 'template:' + t.id,
			type: 'template',
			label: t.name,
			unused: false
		})
	);

	const edges: LayoutEdge[] = [];
	const seenPool: Record<string, true> = {};
	agents.forEach((a) => {
		a.grants.forEach((g) => {
			if (!g.channel) return;
			const key = a.id + '|' + scopeRoot(g.channel);
			if (seenPool[key]) return;
			seenPool[key] = true;
			edges.push({ source: 'agent:' + a.id, target: 'pool:' + scopeRoot(g.channel) });
		});
		a.tools.forEach((tb) => edges.push({ source: 'agent:' + a.id, target: 'tool:' + tb.tool_id }));
		a.lorebooks.forEach((l) => edges.push({ source: 'agent:' + a.id, target: 'lorebook:' + l }));
		if (a.character_id)
			edges.push({ source: 'agent:' + a.id, target: 'character:' + a.character_id });
		if (a.template_id) edges.push({ source: 'agent:' + a.id, target: 'template:' + a.template_id });
	});

	const sig =
		nodes
			.map((n) => n.id)
			.sort()
			.join(',') +
		'|' +
		edges
			.map((e) => e.source + '>' + e.target)
			.sort()
			.join(',') +
		'|' +
		scopeList.join(',');
	if (graphCache.sig === sig) return graphCache.model;

	const laid: LayoutNode[] = [
		...poolRoots.map((r) => ({
			id: 'pool:' + r,
			type: 'pool' as const,
			mass: 3,
			label: r,
			unused: false,
			x: 0,
			y: 0,
			vx: 0,
			vy: 0,
			halfW: POOL_W / 2,
			halfH: (POOL_PAD * 2 + POOL_HEADER_H + poolScopes[r].length * SCOPE_ROW_H) / 2
		})),
		...nodes.map((n) => ({
			id: n.id,
			type: n.type,
			mass: 1,
			label: n.label,
			unused: n.unused,
			x: 0,
			y: 0,
			vx: 0,
			vy: 0,
			halfW: 55,
			halfH: 16
		}))
	];
	relax(laid, edges, GRAPH_W, GRAPH_H);
	const positions: Record<string, { x: number; y: number }> = {};
	laid.forEach((n) => (positions[n.id] = { x: n.x, y: n.y }));

	const model: GraphModel = { nodes, poolRoots, poolScopes, scopeList, positions };
	graphCache = { sig, model };
	return model;
}

/** Ephemeral, per-session view state for the realm editor. The persisted
 * data lives in {@link realmStore}; this only tracks what's on screen. */
class RealmEditorUi {
	tab = $state<EditorTab>('overview');
	expandedAgentId = $state<Snowflake | null>(null);
	selectedAccessNode = $state<{ type: 'agent' | 'scope'; id: string } | null>(null);
	graphFilters = $state<Record<GraphType, boolean>>({
		agent: true,
		scope: true,
		tool: true,
		lorebook: true,
		character: true,
		template: true
	});
	graphSelectedNode = $state<string | null>(null);

	reset() {
		this.tab = 'overview';
		this.expandedAgentId = null;
		this.selectedAccessNode = null;
		this.graphSelectedNode = null;
		this.graphFilters = {
			agent: true,
			scope: true,
			tool: true,
			lorebook: true,
			character: true,
			template: true
		};
	}

	setTab(tab: EditorTab) {
		this.tab = tab;
	}

	toggleAgent(id: Snowflake) {
		this.expandedAgentId = this.expandedAgentId === id ? null : id;
	}

	toggleAccessNode(type: 'agent' | 'scope', id: string) {
		const sel = this.selectedAccessNode;
		this.selectedAccessNode = sel && sel.type === type && sel.id === id ? null : { type, id };
	}

	toggleGraphFilter(type: GraphType) {
		this.graphFilters[type] = !this.graphFilters[type];
	}

	toggleGraphNode(id: string) {
		this.graphSelectedNode = this.graphSelectedNode === id ? null : id;
	}

	selectGraphNode(id: string) {
		const type = id.split(':')[0];
		const key = (type === 'pool' ? 'scope' : type) as GraphType;
		this.graphSelectedNode = id;
		this.graphFilters[key] = true;
	}
}

export const realmEditor = new RealmEditorUi();
