<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { RefreshCw, X } from '@lucide/svelte';

	type Tab = 'state' | 'observations' | 'actions' | 'grants';
	let tab = $state<Tab>('state');

	const inspection = $derived(realmStore.inspection);
	const agent = $derived(inspection ? realmStore.agentById(inspection.agentId) : undefined);
	const namespaces = $derived(Object.entries(inspection?.state ?? {}));
	const playing = $derived(realmStore.inPlaythrough);
</script>

<div class="widget">
	{#if inspection}
		<div class="header">
			<span class="title">{agent?.name ?? 'Agent'}</span>
			<button type="button" title="Refresh" onclick={() => realmStore.refreshInspection()}>
				<RefreshCw size={14} />
			</button>
			<button type="button" title="Close" onclick={() => (realmStore.inspection = null)}>
				<X size={14} />
			</button>
		</div>

		<div class="tabs">
			{#each [['state', 'State'], ['observations', 'Observed'], ['actions', 'Actions'], ['grants', 'Grants']] as const as [id, label] (id)}
				<button type="button" class:active={tab === id} onclick={() => (tab = id)}>
					{label}
				</button>
			{/each}
		</div>

		<div class="content">
			{#if tab === 'state'}
				{#each namespaces as [ns, value] (ns)}
					<div class="entry">
						<span class="key">{ns}</span>
						<pre>{JSON.stringify(value, null, 2)}</pre>
					</div>
				{:else}
					<div class="hint">
						{playing
							? 'This agent can see no world state.'
							: 'State is emergent — start a test run to watch this agent’s view of the pool.'}
					</div>
				{/each}
			{:else if tab === 'observations'}
				{#each inspection.observations as obs (obs.id)}
					<div class="entry">
						<span class="key">{obs.role}</span>
						<p>{obs.content}</p>
					</div>
				{:else}
					<div class="hint">
						{playing
							? 'No observations yet.'
							: 'Observations belong to a playthrough — start a test run to see this agent’s feed.'}
					</div>
				{/each}
			{:else if tab === 'actions'}
				{#each inspection.actions as action (action.tool_id)}
					<div class="entry">
						<span class="key">{action.display_name ?? action.name}</span>
						<p>{action.description}</p>
						<span class="meta">
							event scope: {action.event_scope ?? 'private'}
							{#if action.params.length > 0}
								· params: {action.params.map((p) => p.name).join(', ')}
							{/if}
						</span>
					</div>
				{:else}
					<div class="hint">Speak-only — no bound tools.</div>
				{/each}
			{:else if tab === 'grants'}
				{#each inspection.grants as grant (grant.scope)}
					<div class="entry row">
						<span class="key">{grant.scope}</span>
						<span class="meta">{grant.access === 'read_write' ? 'read/write' : 'read'}</span>
					</div>
				{:else}
					<div class="hint">No grants.</div>
				{/each}
			{/if}
		</div>
	{:else}
		<div class="empty">
			Select an agent to inspect what it knows and can do — its state slice,
			private feed, action space, and grants.
		</div>
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.header {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.title {
		font-weight: 600;
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.header button {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
		background: none;
		border: none;
		color: var(--color-text-secondary);
	}

	.tabs {
		display: flex;
		gap: 2px;
		padding: var(--spacing-xs) var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.tabs button {
		flex: 1;
		font-size: 0.75rem;
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm, 4px);
		background: none;
		border: 1px solid transparent;
		color: var(--color-text-secondary);
	}

	.tabs button.active {
		border-color: var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		color: var(--color-text-primary, inherit);
	}

	.content {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
	}

	.entry {
		display: flex;
		flex-direction: column;
		gap: 2px;
		font-size: 0.85rem;
	}

	.entry.row {
		flex-direction: row;
		justify-content: space-between;
		align-items: center;
	}

	.entry p {
		margin: 0;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.key {
		font-weight: 600;
		font-size: 0.8rem;
		color: var(--color-text-secondary);
		font-family: var(--font-mono, monospace);
	}

	.meta {
		font-size: 0.75rem;
		color: var(--color-text-muted);
	}

	pre {
		margin: 0;
		font-size: 0.75rem;
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm, 4px);
		padding: var(--spacing-xs);
		overflow-x: auto;
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.85rem;
	}

	.empty {
		color: var(--color-text-muted);
		font-size: 0.85rem;
		line-height: 1.5;
		padding: var(--spacing-lg) var(--spacing-md);
	}
</style>
