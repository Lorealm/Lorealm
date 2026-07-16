<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { History, RotateCcw } from '@lucide/svelte';

	let open = $state(false);
	let busyId = $state<string | null>(null);

	const currentId = $derived(realmStore.realm?.current_version_id ?? null);

	async function toggle() {
		open = !open;
		if (open) await realmStore.loadVersions();
	}

	async function restore(versionId: string) {
		if (busyId) return;
		if (!confirm('Restore this version? The current state is kept in history.')) return;
		busyId = versionId;
		try {
			await realmStore.restoreVersion(versionId);
		} finally {
			busyId = null;
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}
</script>

<section class="history">
	<button type="button" class="header" onclick={toggle}>
		<History size={14} />
		<span>Version history</span>
		<span class="chevron">{open ? '▾' : '▸'}</span>
	</button>

	{#if open}
		<div class="versions">
			{#each realmStore.versions as version (version.id)}
				<div class="version" class:current={version.id === currentId}>
					<span class="num">v{version.version_num}</span>
					<span class="date">{formatDate(version.created_at)}</span>
					{#if version.id === currentId}
						<span class="pill">current</span>
					{:else}
						<button
							type="button"
							title="Restore this version (the current one stays in history)"
							disabled={busyId !== null}
							onclick={() => restore(version.id)}
						>
							<RotateCcw size={12} />
							{busyId === version.id ? 'Restoring…' : 'Restore'}
						</button>
					{/if}
				</div>
			{:else}
				<div class="hint">No versions yet.</div>
			{/each}
		</div>
	{/if}
</section>

<style>
	.history {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}

	.header {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		background: none;
		border: none;
		padding: 0;
		color: var(--color-text-secondary);
		font-size: 0.9rem;
		text-align: left;
	}

	.chevron {
		color: var(--color-text-muted);
	}

	.versions {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	.version {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		font-size: 0.85rem;
	}

	.version.current {
		border-color: var(--color-border-strong, var(--color-border-subtle));
	}

	.num {
		font-weight: 600;
		min-width: 3ch;
	}

	.date {
		flex: 1;
		color: var(--color-text-muted);
	}

	.pill {
		font-size: 0.75rem;
		color: var(--color-text-muted);
		border: 1px solid var(--color-border-subtle);
		border-radius: 999px;
		padding: 1px 8px;
	}

	.version button {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: 0.8rem;
		padding: 2px 8px;
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.8rem;
	}
</style>
