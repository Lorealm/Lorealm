<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { RealmAgent } from '$lib/types/realm.js';
	import { Bot, Terminal, User } from '@lucide/svelte';
	import { realmEditor } from './realmEditor.svelte.js';

	const ROW_H = 48;
	const TOP = 28;
	const LEFT_X = 225;
	const RIGHT_X = 655;

	const KIND_ICON = { llm: Bot, player: User, script: Terminal };

	const agents = $derived([...realmStore.agents].sort((a, b) => a.turn_order - b.turn_order));

	const scopeList = $derived.by(() => {
		const list: string[] = [];
		for (const a of agents)
			for (const g of a.grants) if (g.channel && !list.includes(g.channel)) list.push(g.channel);
		return list.sort();
	});

	const sel = $derived(realmEditor.selectedAccessNode);

	function agentActive(a: RealmAgent): boolean {
		if (!sel) return true;
		if (sel.type === 'agent') return sel.id === a.id;
		return a.grants.some((g) => g.channel === sel.id);
	}
	function scopeActive(scope: string): boolean {
		if (!sel) return true;
		if (sel.type === 'scope') return sel.id === scope;
		const a = agents.find((x) => x.id === sel.id);
		return a ? a.grants.some((g) => g.channel === scope) : false;
	}

	function subtitle(a: RealmAgent): string {
		if (a.kind === 'script') return `${a.grants.length} grants`;
		const speech = a.speech_channel ? `→ ${a.speech_channel}` : 'broadcast';
		return `${speech} · ${a.tools.length ? a.tools.length + ' tools' : 'speak-only'}`;
	}

	const canvasHeight = $derived(Math.max(scopeList.length, agents.length, 1) * ROW_H + TOP * 2);

	const edges = $derived.by(() => {
		const list: Array<{
			y1: number;
			y2: number;
			color: string;
			width: number;
			dash: string;
			opacity: number;
		}> = [];
		agents.forEach((a, ai) => {
			a.grants.forEach((g) => {
				if (!g.channel) return;
				const si = scopeList.indexOf(g.channel);
				if (si < 0) return;
				const active =
					!sel ||
					(sel.type === 'agent' && sel.id === a.id) ||
					(sel.type === 'scope' && sel.id === g.channel);
				const rw = g.access === 'read_write';
				list.push({
					y1: TOP + si * ROW_H,
					y2: TOP + ai * ROW_H,
					color: rw ? 'var(--color-primary-light)' : 'var(--color-info)',
					width: rw ? 2.5 : 1.5,
					dash: rw ? '' : '4 3',
					opacity: active ? 0.9 : 0.08
				});
			});
		});
		return list;
	});
</script>

<div class="stage">
	<div class="lede">
		Which agents can read or write which state scopes. Click a scope or agent to trace its
		connections.
	</div>

	{#if scopeList.length > 0}
		<div class="legend">
			<div class="key"><span class="line read"></span>Read</div>
			<div class="key"><span class="line write"></span>Read/Write</div>
		</div>

		<div class="canvasWrap">
			<div class="canvas" style="height:{canvasHeight}px">
				<svg width="880" height={canvasHeight}>
					{#each edges as e, i (i)}
						<line
							x1={LEFT_X}
							y1={e.y1}
							x2={RIGHT_X}
							y2={e.y2}
							stroke={e.color}
							stroke-width={e.width}
							stroke-dasharray={e.dash}
							opacity={e.opacity}
						/>
					{/each}
				</svg>

				{#each scopeList as scope, i (scope)}
					{@const selectedSelf = sel?.type === 'scope' && sel.id === scope}
					<button
						type="button"
						class="scope"
						class:selectedSelf
						class:dim={!scopeActive(scope)}
						style="top:{TOP + i * ROW_H}px"
						onclick={() => realmEditor.toggleAccessNode('scope', scope)}
					>
						{scope}
					</button>
				{/each}

				{#each agents as agent, i (agent.id)}
					{@const Icon = KIND_ICON[agent.kind]}
					{@const selectedSelf = sel?.type === 'agent' && sel.id === agent.id}
					<button
						type="button"
						class="agent"
						class:selectedSelf
						class:dim={!agentActive(agent)}
						style="top:{TOP + i * ROW_H}px"
						onclick={() => realmEditor.toggleAccessNode('agent', agent.id)}
					>
						<div class="agentTop">
							<span class="agentName">{agent.name}</span>
							<Icon size={14} />
						</div>
						<div class="agentSub">{subtitle(agent)}</div>
					</button>
				{/each}
			</div>
		</div>
	{:else}
		<div class="empty">
			No grants defined yet — add grants to agents in the Cast tab to see the access map.
		</div>
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

	.legend {
		display: flex;
		gap: var(--spacing-lg);
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
	}

	.key {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.line {
		display: inline-block;
		width: 16px;
	}

	.line.read {
		height: 2px;
		background: var(--color-info);
	}

	.line.write {
		height: 3px;
		background: var(--color-primary-light);
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
		width: 880px;
	}

	svg {
		position: absolute;
		top: 0;
		left: 0;
	}

	.scope {
		position: absolute;
		left: 20px;
		transform: translateY(-50%);
		max-width: 190px;
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		padding: 4px 8px;
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--color-text-primary);
		border: 1px solid var(--color-border-medium);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.scope.selectedSelf {
		background: var(--color-selected);
		border-color: var(--color-primary);
	}

	.agent {
		position: absolute;
		right: 20px;
		transform: translateY(-50%);
		background: none;
		border: none;
		text-align: right;
		color: var(--color-text-primary);
	}

	.agentTop {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		font-size: var(--font-size-base);
	}

	.agent.selectedSelf .agentTop {
		color: var(--color-text-accent);
	}

	.agentSub {
		font-size: var(--font-size-xs);
		color: var(--color-text-muted);
	}

	.dim {
		color: var(--color-text-muted);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
</style>
