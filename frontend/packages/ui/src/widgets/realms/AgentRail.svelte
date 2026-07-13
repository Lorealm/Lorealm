<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { Bot, Plus, User } from '@lucide/svelte';
	import AgentForm from './AgentForm.svelte';

	let { nextAgentId }: { nextAgentId: string | null } = $props();

	let adding = $state(false);
</script>

<aside class="rail">
	<div class="railHeader">
		<span>Agents</span>
		<button type="button" title="Add agent" onclick={() => (adding = !adding)}>
			<Plus size={14} />
		</button>
	</div>

	{#if adding}
		<AgentForm oncreated={() => (adding = false)} oncancel={() => (adding = false)} />
	{/if}

	<div class="agents">
		{#each realmStore.agents as agent (agent.id)}
			<button
				type="button"
				class="agent"
				class:selected={realmStore.inspection?.agentId === agent.id}
				class:next={agent.id === nextAgentId}
				onclick={() => realmStore.inspectAgent(agent.id)}
				title="Inspect what this agent knows"
			>
				{#if agent.kind === 'player'}
					<User size={16} />
				{:else}
					<Bot size={16} />
				{/if}
				<span class="agentName">{agent.name}</span>
				{#if agent.id === nextAgentId}
					<span class="turnDot" title="Next to act"></span>
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

	.hint {
		color: var(--color-text-muted);
		font-size: 0.85rem;
		padding: var(--spacing-md);
		text-align: center;
	}
</style>
