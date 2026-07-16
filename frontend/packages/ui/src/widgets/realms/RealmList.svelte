<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { onMount } from 'svelte';
	import { Globe, Plus, Trash2 } from '@lucide/svelte';

	let creating = $state(false);
	let newName = $state('');
	let busy = $state(false);

	onMount(async () => {
		await realmStore.loadList();
	});

	async function create() {
		const name = newName.trim();
		if (!name || busy) return;
		busy = true;
		try {
			const realm = await realmStore.createRealm(name);
			creating = false;
			newName = '';
			await realmStore.openRealm(realm.id);
		} finally {
			busy = false;
		}
	}

	async function remove(e: MouseEvent, id: string) {
		e.stopPropagation();
		await realmStore.deleteRealm(id);
	}
</script>

<div class="page">
	<div class="topbar">
		<h2>Realms</h2>
		<div class="actions">
			{#if creating}
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					placeholder="Realm name"
					bind:value={newName}
					autofocus
					onkeydown={(e) => {
						if (e.key === 'Enter') create();
						if (e.key === 'Escape') {creating = false; newName = ''};
					}}
				/>
				<button type="button" class="primary" onclick={create} disabled={busy || !newName.trim()}>
					Create
				</button>
				<button type="button" onclick={() => ((creating = false), (newName = ''))}>Cancel</button>
			{:else}
				<button type="button" class="primary" onclick={() => (creating = true)}>
					<Plus size={16} />
					New
				</button>
			{/if}
		</div>
	</div>

	{#if realmStore.summaries.length === 0}
		<div class="empty">No realms yet — create one to get started.</div>
	{:else}
		<div class="list">
			{#each realmStore.summaries as realm (realm.id)}
				<div
					class="card"
					role="button"
					tabindex="0"
					onclick={() => realmStore.openRealm(realm.id)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') realmStore.openRealm(realm.id);
					}}
				>
					<Globe size={20} />
					<div class="cardBody">
						<span class="name">{realm.name}</span>
						{#if realm.tags.length > 0}
							<span class="tags">
								{#each realm.tags as tag (tag)}
									<span class="tag">{tag}</span>
								{/each}
							</span>
						{/if}
					</div>
					<button
						type="button"
						class="delete"
						title="Delete realm"
						onclick={(e) => remove(e, realm.id)}
					>
						<Trash2 size={16} />
					</button>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	.topbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		backdrop-filter: var(--backdrop-blur);
		position: sticky;
		top: 0;
		z-index: var(--z-index-sticky);
	}

	.topbar h2 {
		margin: 0;
		font-size: var(--font-size-lg, 1.1rem);
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
	}

	.actions input {
		width: 220px;
	}

	.topbar button {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}

	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}

	.list {
		flex: 1 1 auto;
		min-height: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		grid-auto-rows: max-content;
		gap: var(--grid-gap, var(--spacing-md));
		padding: var(--spacing-lg);
		overflow-y: auto;
		align-content: start;
	}

	.card {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-md) var(--spacing-lg);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		background-color: var(--color-bg-elevated);
		cursor: pointer;
		text-align: left;
	}

	.card:hover {
		border-color: var(--color-border-strong, var(--color-border-subtle));
		box-shadow: var(--shadow-md);
	}

	.cardBody {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}

	.name {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}

	.tag {
		font-size: 0.7rem;
		color: var(--color-text-muted);
		border: 1px solid var(--color-border-subtle);
		border-radius: 999px;
		padding: 1px 8px;
	}

	.delete {
		background: none;
		border: none;
		color: var(--color-text-muted);
		padding: var(--spacing-xs);
	}

	.delete:hover {
		color: var(--color-danger, #e5484d);
	}

	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
		text-align: center;
	}
</style>
