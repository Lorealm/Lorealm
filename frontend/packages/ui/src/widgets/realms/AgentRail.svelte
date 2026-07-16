<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { RealmAgent } from '$lib/types/realm.js';
	import { Bot, ChevronDown, ChevronUp, CodeXml, Plus, Trash2, User } from '@lucide/svelte';

	let {
		selectedId = null,
		onselect,
		oncreate
	}: {
		/** Editor mode: the agent currently open in the editor. */
		selectedId?: string | null;
		/** Editor mode: pick an agent to edit (also feeds the inspector). */
		onselect?: (agent: RealmAgent) => void;
		/** Editor mode: start authoring a new agent. */
		oncreate?: () => void;
	} = $props();

	const playing = $derived(realmStore.inPlaythrough);
	const nextAgentId = $derived(realmStore.nextAgent?.id ?? null);

	function pick(agent: RealmAgent) {
		if (playing) {
			// Play mode: toggle the knowledge inspection.
			void realmStore.inspectAgent(agent.id);
		} else {
			onselect?.(agent);
			void realmStore.selectInspection(agent.id);
		}
	}

	async function remove(e: MouseEvent, id: string) {
		e.stopPropagation();
		await realmStore.deleteAgent(id);
	}

	async function move(e: MouseEvent, id: string, delta: -1 | 1) {
		e.stopPropagation();
		await realmStore.moveAgent(id, delta);
	}
</script>

<aside class="rail">
	<div class="railHeader">
		<span>{playing ? 'Cast' : 'Agents'}</span>
		{#if !playing}
			<button type="button" title="Add agent" onclick={() => oncreate?.()}>
				<Plus size={14} />
			</button>
		{/if}
	</div>

	<div class="agents">
		{#each realmStore.agents as agent, index (agent.id)}
			<button
				type="button"
				class="agent"
				class:selected={playing
					? realmStore.inspection?.agentId === agent.id
					: selectedId === agent.id}
				class:next={agent.id === nextAgentId}
				onclick={() => pick(agent)}
				title={playing ? 'Inspect what this agent knows' : 'Edit this agent'}
			>
				{#if agent.kind === 'player'}
					<User size={16} />
				{:else if agent.kind === 'script'}
				    <CodeXml size={16} />
				{:else}
					<Bot size={16} />
				{/if}
				<span class="agentName">{agent.name}</span>
				{#if agent.id === nextAgentId}
					<span class="turnDot" title="Next to act"></span>
				{/if}
				{#if !playing}
					<span
						class="rowAction"
						class:hiddenAction={index === 0}
						role="button"
						tabindex="-1"
						title="Move up in the turn order"
						onclick={(e) => move(e, agent.id, -1)}
						onkeydown={(e) => e.stopPropagation()}
					>
						<ChevronUp size={14} />
					</span>
					<span
						class="rowAction"
						class:hiddenAction={index === realmStore.agents.length - 1}
						role="button"
						tabindex="-1"
						title="Move down in the turn order"
						onclick={(e) => move(e, agent.id, 1)}
						onkeydown={(e) => e.stopPropagation()}
					>
						<ChevronDown size={14} />
					</span>
					<span
						class="rowAction delete"
						role="button"
						tabindex="-1"
						title="Remove agent from the preset"
						onclick={(e) => remove(e, agent.id)}
						onkeydown={(e) => e.stopPropagation()}
					>
						<Trash2 size={14} />
					</span>
				{/if}
			</button>
		{:else}
			<div class="hint">No agents yet. Add an LLM agent and a player to start.</div>
		{/each}
	</div>
</aside>

<style>
	.rail {
		display: flex;
		flex-direction: column;
		width: 240px;
		flex-shrink: 0;
		border-right: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		min-height: 0;
	}

	.railHeader {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		font-weight: 600;
		font-size: 0.85rem;
		color: var(--color-text-secondary);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.railHeader button {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
	}

	.agents {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--spacing-sm);
		overflow-y: auto;
		min-height: 0;
	}

	.agent {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid transparent;
		border-radius: var(--radius-md, 8px);
		background: none;
		text-align: left;
		color: var(--color-text-primary, inherit);
	}

	.agent:hover {
		background-color: var(--color-bg-elevated);
	}

	.agent.selected {
		border-color: var(--color-border-strong, var(--color-border-subtle));
		background-color: var(--color-bg-elevated);
	}

	.agentName {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.turnDot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background-color: var(--color-success, #4caf7d);
		flex-shrink: 0;
	}

	.rowAction {
		display: none;
		align-items: center;
		color: var(--color-text-muted);
		flex-shrink: 0;
	}

	.agent:hover .rowAction {
		display: flex;
	}

	.agent:hover .rowAction.hiddenAction {
		visibility: hidden;
	}

	.rowAction:hover {
		color: var(--color-text-primary, inherit);
	}

	.delete:hover {
		color: var(--color-danger, #e5484d);
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.85rem;
		padding: var(--spacing-md);
		text-align: center;
	}
</style>
