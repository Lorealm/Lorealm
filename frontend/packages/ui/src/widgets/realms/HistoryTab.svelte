<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { onMount } from 'svelte';
	import Badge from './Badge.svelte';

	onMount(() => {
		void realmStore.loadVersions();
	});

	const currentId = $derived(realmStore.realm?.current_version_id);
	const versions = $derived([...realmStore.versions].sort((a, b) => b.version_num - a.version_num));

	function formatDate(iso: string): string {
		const d = new Date(iso);
		return Number.isNaN(d.getTime()) ? iso : d.toLocaleDateString();
	}
</script>

<div class="stage">
	{#each versions as version (version.id)}
		{@const current = version.id === currentId}
		<div class="card">
			<div class="num">v{version.version_num}</div>
			<div class="meta">
				<div class="date">{formatDate(version.created_at)}</div>
			</div>
			{#if current}
				<Badge tone="success">Current</Badge>
			{:else}
				<button
					type="button"
					class="secondary"
					onclick={() => void realmStore.restoreVersion(version.id)}
				>
					Restore
				</button>
			{/if}
		</div>
	{:else}
		<div class="empty">No version history yet.</div>
	{/each}
</div>

<style>
	.stage {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		max-width: var(--width-content-max);
	}

	.card {
		display: flex;
		align-items: center;
		gap: var(--spacing-lg);
		padding: var(--spacing-md);
		background: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
	}

	.num {
		font-family: var(--font-family-heading);
		font-weight: 600;
		width: 48px;
	}

	.meta {
		flex: 1;
		min-width: 0;
	}

	.date {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-base);
	}
</style>
