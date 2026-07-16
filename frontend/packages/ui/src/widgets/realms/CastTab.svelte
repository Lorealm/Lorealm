<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { AgentKind } from '$lib/types/realm.js';
	import AgentCard from './AgentCard.svelte';
	import { realmEditor } from './realmEditor.svelte.js';

	let newKind = $state<AgentKind>('llm');
	let adding = $state(false);

	const agents = $derived([...realmStore.agents].sort((a, b) => a.turn_order - b.turn_order));

	async function addAgent() {
		if (adding) return;
		adding = true;
		try {
			const label = newKind === 'llm' ? 'LLM' : newKind === 'player' ? 'Player' : 'Script';
			const saved = await realmStore.createAgent({
				name: `New ${label}`,
				kind: newKind,
				script: newKind === 'script' ? '$[set_var("state:placeholder", "edit me")]' : null
			});
			if (saved) realmEditor.expandedAgentId = saved.id;
		} finally {
			adding = false;
		}
	}
</script>

<div class="stage">
	<div class="toolbar">
		<select bind:value={newKind}>
			<option value="llm">LLM</option>
			<option value="player">Player</option>
			<option value="script">Script</option>
		</select>
		<button type="button" class="primary" onclick={addAgent} disabled={adding}>+ Add agent</button>
	</div>

	{#each agents as agent, i (agent.id)}
		<AgentCard {agent} index={i} total={agents.length} />
	{:else}
		<div class="empty">No agents yet — add one to build the cast.</div>
	{/each}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		max-width: var(--width-content-max);
	}

	.toolbar {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--spacing-sm);
	}

	.toolbar select {
		width: 140px;
	}

	.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
</style>
