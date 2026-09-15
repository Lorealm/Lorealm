<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { BookOpen, Bot, Database, FileText, Terminal, User, Wrench } from '@lucide/svelte';
	import Tag from './Tag.svelte';
	import {
		buildGraph,
		GRAPH_H,
		GRAPH_W,
		POOL_HEADER_H,
		POOL_PAD,
		POOL_W,
		realmEditor,
		scopeRoot,
		SCOPE_ROW_H,
		type GraphType
	} from './realmEditor.svelte.js';

	const TYPE_COLOR: Record<GraphType, string> = {
		agent: 'var(--color-text-accent)',
		scope: 'var(--color-warning)',
		tool: 'var(--color-success)',
		lorebook: 'var(--color-info)',
		character: 'var(--color-primary-light)',
		template: 'var(--color-text-secondary)'
	};

	const FILTER_DEFS: Array<{ type: GraphType; label: string }> = [
		{ type: 'agent', label: 'Agents' },
		{ type: 'scope', label: 'State Pools' },
		{ type: 'tool', label: 'Tools' },
		{ type: 'lorebook', label: 'Lorebooks' },
		{ type: 'character', label: 'Characters' },
		{ type: 'template', label: 'Templates' }
	];

	const TYPE_ICON = { tool: Wrench, lorebook: BookOpen, character: User, template: FileText };
	const KIND_ICON = { llm: Bot, player: User, script: Terminal };

	const realm = $derived(realmStore.realm!);
	const model = $derived(buildGraph(realm));
	const filters = $derived(realmEditor.graphFilters);
	const gsel = $derived(realmEditor.graphSelectedNode);

	const agentKind = (id: string) => realm.definition.agents.find((a) => a.handle === id)?.kind ?? 'llm';

	function nodeIcon(type: GraphType, id: string) {
		if (type === 'agent') return KIND_ICON[agentKind(id.slice('agent:'.length))];
		return TYPE_ICON[type as 'tool' | 'lorebook' | 'character' | 'template'];
	}

	// The neighbourhood the selected node lights up.
	const activeIds = $derived.by<Record<string, true> | null>(() => {
		if (!gsel) return null;
		const ids: Record<string, true> = { [gsel]: true };
		const add = (id: string) => (ids[id] = true);
		const [gt, ...rest] = gsel.split(':');
		const gid = rest.join(':');
		const agents = realm.definition.agents;
		if (gt === 'agent') {
			const a = agents.find((x) => x.handle === gid);
			if (a) {
				a.grants.forEach((g) => {
					if (g.channel) {
						add('scope:' + g.channel);
						add('pool:' + scopeRoot(g.channel));
					}
				});
				a.tools.forEach((tb) => add('tool:' + tb.tool_id));
				a.lorebooks.forEach((l) => add('lorebook:' + l));
				if (a.character_id) add('character:' + a.character_id);
				if (a.template_id) add('template:' + a.template_id);
			}
		} else if (gt === 'scope') {
			add('pool:' + scopeRoot(gid));
			agents.forEach((a) => {
				if (a.grants.some((g) => g.channel === gid)) add('agent:' + a.handle);
			});
		} else if (gt === 'pool') {
			(model.poolScopes[gid] ?? []).forEach((sc) => add('scope:' + sc));
			agents.forEach((a) => {
				if (a.grants.some((g) => g.channel && scopeRoot(g.channel) === gid)) add('agent:' + a.handle);
			});
		} else if (gt === 'tool') {
			agents.forEach((a) => a.tools.some((tb) => tb.tool_id === gid) && add('agent:' + a.handle));
		} else if (gt === 'lorebook') {
			agents.forEach((a) => a.lorebooks.includes(gid) && add('agent:' + a.handle));
		} else if (gt === 'character') {
			agents.forEach((a) => a.character_id === gid && add('agent:' + a.handle));
		} else if (gt === 'template') {
			agents.forEach((a) => a.template_id === gid && add('agent:' + a.handle));
		}
		return ids;
	});
	const isActive = (id: string) => !activeIds || activeIds[id] === true;

	const pos = (id: string) => model.positions[id] ?? { x: GRAPH_W / 2, y: GRAPH_H / 2 };

	// Pool boxes plus the attach point for each scope chip (for edge endpoints).
	const pools = $derived.by(() => {
		const scopeAttach: Record<string, { x: number; y: number }> = {};
		const boxes = filters.scope
			? model.poolRoots.map((root) => {
					const p = pos('pool:' + root);
					const scopes = model.poolScopes[root];
					const boxH = POOL_PAD * 2 + POOL_HEADER_H + scopes.length * SCOPE_ROW_H;
					const left = p.x - POOL_W / 2;
					const top = p.y - boxH / 2;
					const poolActive =
						!gsel || isActive('pool:' + root) || scopes.some((sc) => isActive('scope:' + sc));
					const chips = scopes.map((sc, i) => {
						const cy = top + POOL_PAD + POOL_HEADER_H + i * SCOPE_ROW_H + SCOPE_ROW_H / 2;
						scopeAttach[sc] = { x: left, y: cy };
						return {
							scope: sc,
							leaf: sc === root ? '(all)' : sc.slice(root.length + 1),
							active: !gsel || isActive('scope:' + sc),
							selected: gsel === 'scope:' + sc
						};
					});
					return { root, left, top, width: POOL_W, active: poolActive, chips };
				})
			: [];
		return { boxes, scopeAttach };
	});

	const edges = $derived.by(() => {
		const list: Array<{
			x1: number;
			y1: number;
			x2: number;
			y2: number;
			color: string;
			width: number;
			dash: string;
			opacity: number;
		}> = [];
		const attach = pools.scopeAttach;
		realm.definition.agents.forEach((a) => {
			if (!filters.agent) return;
			const sp = pos('agent:' + a.handle);
			const push = (
				targetId: string,
				tp: { x: number; y: number },
				color: string,
				width: number,
				dash: string
			) => {
				const dim = !!gsel && !(isActive('agent:' + a.handle) && isActive(targetId));
				list.push({
					x1: sp.x,
					y1: sp.y,
					x2: tp.x,
					y2: tp.y,
					color,
					width,
					dash,
					opacity: dim ? 0.05 : 0.7
				});
			};
			if (filters.scope)
				a.grants.forEach((g) => {
					if (!g.channel) return;
					const tp = attach[g.channel];
					if (!tp) return;
					const rw = g.access === 'read_write';
					push(
						'scope:' + g.channel,
						tp,
						rw ? 'var(--color-primary-light)' : 'var(--color-warning)',
						rw ? 2.5 : 1.5,
						rw ? '' : '4 3'
					);
				});
			if (filters.tool)
				a.tools.forEach((tb) => {
					if (model.positions['tool:' + tb.tool_id])
						push('tool:' + tb.tool_id, pos('tool:' + tb.tool_id), TYPE_COLOR.tool, 1.5, '');
				});
			if (filters.lorebook)
				a.lorebooks.forEach((l) => {
					if (model.positions['lorebook:' + l])
						push('lorebook:' + l, pos('lorebook:' + l), TYPE_COLOR.lorebook, 1.5, '');
				});
			if (filters.character && a.character_id && model.positions['character:' + a.character_id])
				push(
					'character:' + a.character_id,
					pos('character:' + a.character_id),
					TYPE_COLOR.character,
					1.5,
					''
				);
			if (filters.template && a.template_id && model.positions['template:' + a.template_id])
				push(
					'template:' + a.template_id,
					pos('template:' + a.template_id),
					TYPE_COLOR.template,
					1.5,
					''
				);
		});
		return list;
	});

	const nodeViews = $derived(
		model.nodes
			.filter((n) => filters[n.type])
			.map((n) => ({
				...n,
				pos: pos(n.id),
				color: TYPE_COLOR[n.type],
				dim: !!gsel && !isActive(n.id)
			}))
	);

	const counts = $derived({
		agent: realm.definition.agents.length,
		scope: model.scopeList.length,
		tool: model.nodes.filter((n) => n.type === 'tool').length,
		lorebook: model.nodes.filter((n) => n.type === 'lorebook').length,
		character: model.nodes.filter((n) => n.type === 'character').length,
		template: model.nodes.filter((n) => n.type === 'template').length
	});
</script>

<div class="stage">
	<div class="lede">
		Everything in this realm and how it connects: agents, state pools, tools, lorebooks, characters,
		and templates. Click a filter to hide a type, click a node to trace its neighborhood. Dashed
		nodes are imported but unused.
	</div>

	<div class="filters">
		{#each FILTER_DEFS as f (f.type)}
			<Tag selected={filters[f.type]} onclick={() => realmEditor.toggleGraphFilter(f.type)}>
				<span class="dot" style="background:{TYPE_COLOR[f.type]}"></span>{f.label} ({counts[
					f.type
				]})
			</Tag>
		{/each}
	</div>

	{#if model.nodes.length > 0}
		<div class="canvasWrap">
			<div class="canvas" style="width:{GRAPH_W}px;height:{GRAPH_H}px">
				<svg width={GRAPH_W} height={GRAPH_H}>
					{#each edges as e, i (i)}
						<line
							x1={e.x1}
							y1={e.y1}
							x2={e.x2}
							y2={e.y2}
							stroke={e.color}
							stroke-width={e.width}
							stroke-dasharray={e.dash}
							opacity={e.opacity}
						/>
					{/each}
				</svg>

				{#each pools.boxes as p (p.root)}
					<div
						class="pool"
						style="left:{p.left}px;top:{p.top}px;width:{p.width}px;opacity:{p.active ? 1 : 0.22}"
					>
						<button
							type="button"
							class="poolHead"
							onclick={() => realmEditor.selectGraphNode('pool:' + p.root)}
						>
							<Database size={13} color="var(--color-warning)" />
							<span>{p.root}</span>
						</button>
						{#each p.chips as chip (chip.scope)}
							<button
								type="button"
								class="scopeChip"
								class:selected={chip.selected}
								style="opacity:{chip.active ? 1 : 0.3}"
								onclick={() => realmEditor.selectGraphNode('scope:' + chip.scope)}
							>
								{chip.leaf}
							</button>
						{/each}
					</div>
				{/each}

				{#each nodeViews as n (n.id)}
					{@const Icon = nodeIcon(n.type, n.id)}
					<button
						type="button"
						class="node"
						style="left:{n.pos.x}px;top:{n.pos.y}px;opacity:{n.dim ? 0.22 : 1}"
						onclick={() => realmEditor.toggleGraphNode(n.id)}
					>
						<span
							class="pill"
							style="border-color:{n.color};border-style:{n.unused ? 'dashed' : 'solid'}"
						>
							<Icon size={12} color={n.color} />
							<span class="pillLabel">{n.label}</span>
						</span>
						{#if n.unused}<span class="unused">unused</span>{/if}
					</button>
				{/each}
			</div>
		</div>
	{:else}
		<div class="empty">Nothing to show yet — add agents and imports first.</div>
	{/if}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.lede {
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
		line-height: var(--line-height-normal);
	}

	.filters {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
	}

	.dot {
		display: inline-block;
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		margin-right: 6px;
	}

	.canvasWrap {
		width: 100%;
		overflow-x: auto;
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
	}

	.canvas {
		position: relative;
	}

	svg {
		position: absolute;
		top: 0;
		left: 0;
	}

	.pool {
		position: absolute;
		background: var(--color-bg-elevated-2);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-lg);
		padding: 8px;
	}

	.poolHead {
		display: flex;
		align-items: center;
		gap: 6px;
		height: 26px;
		padding: 0 4px;
		width: 100%;
		background: none;
		border: none;
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-sm);
		color: var(--color-warning-text);
	}

	.scopeChip {
		display: flex;
		align-items: center;
		height: 24px;
		padding: 0 6px;
		width: 100%;
		background: transparent;
		border: none;
		border-radius: var(--radius-sm);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		color: var(--color-text-primary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.scopeChip:hover {
		background: var(--color-bg-elevated-3);
	}

	.scopeChip.selected {
		background: var(--color-selected);
	}

	.node {
		position: absolute;
		transform: translate(-50%, -50%);
		display: flex;
		flex-direction: column;
		align-items: center;
		max-width: 120px;
		background: none;
		border: none;
		padding: 0;
	}

	.pill {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		padding: 5px 10px;
		border-radius: var(--radius-full);
		background: var(--color-bg-elevated-3);
		border-width: 1px;
		white-space: nowrap;
	}

	.pillLabel {
		font-size: var(--font-size-xs);
		color: var(--color-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 90px;
		white-space: nowrap;
	}

	.unused {
		font-size: 9px;
		color: var(--color-text-muted);
		margin-top: 2px;
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
</style>
