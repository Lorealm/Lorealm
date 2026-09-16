<script lang="ts">
	import { FileCode2, Plus, Trash2 } from '@lucide/svelte';
	import { blueprintStore } from '$lib/stores/blueprint.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { realmDraft } from './draft.svelte.js';
	import { panes } from './panes.svelte.js';
	import { toWtn } from './serialize.js';

	let busy = $state(false);

	$effect(() => {
		void blueprintStore.loadList();
	});

	async function create() {
		if (busy) return;
		busy = true;
		try {
			const name = 'Untitled Blueprint';
			const made = await blueprintStore.create(name, toWtn(name, '', [], []).source);
			if (made) start(name, '');
		} finally {
			busy = false;
		}
	}

	async function open(id: Snowflake) {
		await blueprintStore.open(id);
		const current = blueprintStore.current;
		if (current) start(current.name, current.description);
	}

	function start(name: string, description: string) {
		realmDraft.reset();
		panes.reset();
		realmDraft.name = name;
		realmDraft.description = description;
	}

	async function remove(event: MouseEvent, id: Snowflake) {
		event.stopPropagation();
		await blueprintStore.delete(id);
	}
</script>

<div class="section">
	<div class="head">
		<h2>Blueprints</h2>
		<button type="button" class="new" onclick={create} disabled={busy}>
			<Plus size={13} /> New blueprint
		</button>
	</div>

	<div class="grid">
		{#each blueprintStore.summaries as blueprint (blueprint.id)}
			<div
				class="card"
				role="button"
				tabindex="0"
				onclick={() => open(blueprint.id)}
				onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && open(blueprint.id)}
			>
				<div class="cardTop">
					<FileCode2 size={14} />
					<span class="name">{blueprint.name}</span>
					<button
						type="button"
						class="del"
						title="Delete blueprint"
						aria-label="Delete {blueprint.name}"
						onclick={(e) => remove(e, blueprint.id)}
					>
						<Trash2 size={14} />
					</button>
				</div>
				{#if blueprint.description}
					<p class="desc">{blueprint.description}</p>
				{/if}
			</div>
		{:else}
			<p class="empty">No blueprints yet. A blueprint builds realms from a few inputs.</p>
		{/each}
	</div>
</div>

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h4);
		font-weight: var(--font-weight-semibold);
	}
	.new {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.new:hover:not(:disabled) {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--spacing-md);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-md);
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		color: var(--color-text-secondary);
	}
	.card:hover {
		border-color: var(--color-border-active);
		color: var(--color-text-primary);
	}
	.cardTop {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-md);
	}
	.del {
		flex: 0 0 auto;
		display: flex;
		padding: 2px;
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.del:hover {
		color: var(--color-error-text);
	}
	.desc {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
