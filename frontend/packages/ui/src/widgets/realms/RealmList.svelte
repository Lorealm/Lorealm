<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { Plus, Trash2 } from '@lucide/svelte';
	import Tag from './Tag.svelte';

	let busy = $state(false);

	async function createRealm() {
		if (busy) return;
		busy = true;
		try {
			const realm = await realmStore.createRealm('Untitled Realm');
			navigator.start().realms(realm.id);
		} finally {
			busy = false;
		}
	}

	async function remove(e: MouseEvent, id: Snowflake) {
		e.stopPropagation();
		await realmStore.deleteRealm(id);
	}
</script>

<div class="page">
	<div class="head">
		<h2>Realms</h2>
	</div>
	<div class="grid">
		{#each realmStore.summaries as realm (realm.id)}
			<div
				class="card"
				role="button"
				tabindex="0"
				onclick={() => navigator.start().realms(realm.id)}
				onkeydown={(e) => {
					if (e.key === 'Enter' || e.key === ' ') navigator.start().realms(realm.id);
				}}
			>
				<div class="cardTop">
					<div class="name">{realm.name}</div>
					<button
						type="button"
						class="del"
						title="Delete realm"
						onclick={(e) => remove(e, realm.id)}
					>
						<Trash2 size={14} />
					</button>
				</div>
				{#if realm.tags.length > 0}
					<div class="tags">
						{#each realm.tags as tag (tag)}
							<Tag>{tag}</Tag>
						{/each}
					</div>
				{/if}
			</div>
		{/each}
		<button type="button" class="new" onclick={createRealm} disabled={busy}>
			<Plus size={22} />
			<div>New realm</div>
		</button>
	</div>
</div>

<style>
	.page {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-2xl);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	h2 {
		margin: 0;
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--grid-gap);
	}

	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		min-height: 120px;
		padding: var(--spacing-lg);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		cursor: pointer;
		text-align: left;
	}

	.card:hover {
		border-color: var(--color-border-medium);
		box-shadow: var(--shadow-md);
	}

	.cardTop {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}

	.name {
		font-family: var(--font-family-heading);
		font-weight: 600;
		font-size: var(--font-size-md);
		color: var(--color-text-primary);
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.del {
		display: flex;
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: 2px;
		opacity: 0;
	}

	.card:hover .del {
		opacity: 1;
	}

	.del:hover {
		color: var(--color-danger, #e5484d);
	}

	.tags {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.new {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-sm);
		min-height: 120px;
		border: 1px dashed var(--color-border-medium);
		border-radius: var(--radius-lg);
		background: none;
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
	}

	.new:hover:not(:disabled) {
		border-color: var(--color-border-active);
		color: var(--color-text-primary);
	}
</style>
