<script lang="ts">
	import { FileCode2, Play, Trash2 } from '@lucide/svelte';
	import { useApi } from '$lib/api/client.js';
	import { blueprintStore } from '$lib/stores/blueprint.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import type { ConversationMetadata } from '$lib/types/conversation.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { RealmSummary } from '$lib/types/realm.js';
	import PlayMenu from './PlayMenu.svelte';

	const api = useApi();

	let conversations = $state<ConversationMetadata[]>([]);
	let playing = $state<RealmSummary | null>(null);

	$effect(() => {
		void realmStore.loadList();
		void blueprintStore.loadList();
		api.chat
			.list()
			.then((all) => (conversations = all.filter((c) => c.realm_id !== null)))
			.catch(() => (conversations = []));
	});

	const blueprintName = (id: Snowflake) =>
		blueprintStore.summaries.find((b) => b.id === id)?.name ?? 'a deleted blueprint';

	const playthroughs = (realmId: Snowflake) => conversations.filter((c) => c.realm_id === realmId);

	async function remove(event: MouseEvent, id: Snowflake) {
		event.stopPropagation();
		await realmStore.deleteRealm(id);
	}
</script>

<div class="section">
	<div class="grid">
		{#each realmStore.summaries as realm (realm.id)}
			{@const runs = playthroughs(realm.id)}
			<div class="card">
				<div class="cardTop">
					<span class="name">{realm.name || 'Untitled Realm'}</span>
					<button
						type="button"
						class="del"
						title="Delete realm"
						aria-label="Delete {realm.name}"
						onclick={(e) => remove(e, realm.id)}
					>
						<Trash2 size={14} />
					</button>
				</div>

				{#if realm.description}
					<p class="desc">{realm.description}</p>
				{/if}

				<div class="meta">
					{#if realm.origin}
						<span class="from">
							<FileCode2 size={11} />
							{blueprintName(realm.origin.blueprint_id)}
						</span>
					{:else}
						<span class="from imported">imported</span>
					{/if}
					<span class="runs">
						{runs.length}
						{runs.length === 1 ? 'playthrough' : 'playthroughs'}
					</span>
				</div>

				<button type="button" class="play" onclick={() => (playing = realm)}>
					<Play size={13} /> Play
				</button>
			</div>
		{:else}
			<p class="empty">No realms yet. Import one or build one from a blueprint.</p>
		{/each}
	</div>
</div>

{#if playing}
	{@const target = playing}
	<PlayMenu
		realm={target}
		conversations={playthroughs(target.id)}
		onclose={() => (playing = null)}
	/>
{/if}

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: var(--spacing-md);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		color: var(--color-text-secondary);
	}
	.card:hover {
		border-color: var(--color-border-active);
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
		color: var(--color-text-primary);
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
	.meta {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex-wrap: wrap;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.from {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.from.imported {
		font-style: italic;
	}
	.runs {
		margin-left: auto;
	}
	.play {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		background: var(--gradient-primary);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-white);
		font-size: var(--font-size-sm);
	}
	.play:hover {
		box-shadow: var(--shadow-glow);
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
