<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { AgentKind } from '$lib/types/realm.js';
	import AgentCard from './AgentCard.svelte';
	import { catalogItems, realmEditor } from './realmEditor.svelte.js';

	let newKind = $state<AgentKind>('llm');
	let newTemplateId = $state<Snowflake | ''>('');
	let newProfileId = $state<Snowflake | ''>('');
	let adding = $state(false);

	const agents = $derived([...realmStore.agents].sort((a, b) => a.turn_order - b.turn_order));

	const templates = $derived(catalogItems('template'));
	const profiles = $derived(catalogItems('connection_profile'));

	const canAdd = $derived(newKind !== 'llm' || (newTemplateId !== '' && newProfileId !== ''));

	/** First `<kind>_<n>` handle not taken by the cast. */
	function nextHandle(kind: AgentKind): string {
		const taken = new Set(realmStore.agents.map((a) => a.handle));
		for (let i = 1; ; i++) {
			if (!taken.has(`${kind}_${i}`)) return `${kind}_${i}`;
		}
	}

	async function addAgent() {
		if (adding || !canAdd) return;
		adding = true;
		try {
			const label = newKind === 'llm' ? 'LLM' : newKind === 'player' ? 'Player' : 'Script';
			const saved = await realmStore.createAgent({
				name: `New ${label}`,
				handle: nextHandle(newKind),
				kind: newKind,
				template_id: newKind === 'llm' ? newTemplateId || null : null,
				profile_id: newKind === 'llm' ? newProfileId || null : null,
				script: newKind === 'script' ? '$[set_var("state:placeholder", "edit me")]' : null
			});
			if (saved) {
				realmEditor.expandedHandle = saved.handle;
				newTemplateId = '';
				newProfileId = '';
			}
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
		{#if newKind === 'llm'}
			<select bind:value={newTemplateId}>
				<option value="">Template…</option>
				{#each templates as t (t.id)}
					<option value={t.id}>{t.name}</option>
				{/each}
			</select>
			<select bind:value={newProfileId}>
				<option value="">Connection profile…</option>
				{#each profiles as p (p.id)}
					<option value={p.id}>{p.name}</option>
				{/each}
			</select>
		{/if}
		<button type="button" class="primary" onclick={addAgent} disabled={adding || !canAdd}>
			+ Add agent
		</button>
	</div>
	{#if newKind === 'llm' && (templates.length === 0 || profiles.length === 0)}
		<div class="hint">Import a template and a connection profile first.</div>
	{/if}

	{#each agents as agent, i (agent.handle)}
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

	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-align: right;
	}
</style>
