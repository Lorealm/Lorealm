<script lang="ts">
	import type { DraftEntry, SaveState } from '$lib/stores/lorebook.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { sortEntries, SORT_KEYS, type SortKey } from '$lib/weaver/entryOrdering.js';

	let {
		entries,
		selectedId,
		defaultPriority,
		onSelect,
		onToggle,
		onDelete,
		entryState
	}: {
		entries: DraftEntry[];
		selectedId: Snowflake | null;
		defaultPriority: number;
		onSelect: (id: Snowflake) => void;
		onToggle: (id: Snowflake) => void;
		onDelete: (id: Snowflake) => void;
		entryState: (id: Snowflake) => SaveState;
	} = $props();

	const PAGE_SIZE = 25;

	let sortKey = $state<SortKey>('name');
	let page = $state(0);

	const sorted = $derived(sortEntries(entries, sortKey, { priority: defaultPriority }));
	const pageCount = $derived(Math.max(1, Math.ceil(sorted.length / PAGE_SIZE)));
	const safePage = $derived(Math.min(page, pageCount - 1));
	const pageItems = $derived(sorted.slice(safePage * PAGE_SIZE, safePage * PAGE_SIZE + PAGE_SIZE));

	function changeSort(key: SortKey) {
		sortKey = key;
		page = 0;
	}
	const label = (e: DraftEntry) => e.meta.name?.trim() || e.meta.id;
</script>

<div class="entry-list">
	<div class="controls">
		<label class="sort">
			Sort
			<select value={sortKey} onchange={(e) => changeSort(e.currentTarget.value as SortKey)}>
				{#each SORT_KEYS as s (s.key)}
					<option value={s.key}>{s.label}</option>
				{/each}
			</select>
		</label>
	</div>

	{#if sorted.length === 0}
		<p class="empty">No entries yet.</p>
	{:else}
		<ul class="rows">
			{#each pageItems as entry (entry.snowflake)}
				{@const invalid = entryState(entry.snowflake).status === 'error'}
				{@const enabled = entry.meta.enabled ?? true}
				<li class="row" class:selected={entry.snowflake === selectedId} class:disabled={!enabled}>
					<input
						type="checkbox"
						class="toggle"
						checked={enabled}
						aria-label="Enabled"
						onclick={(e) => {
							e.stopPropagation();
							onToggle(entry.snowflake);
						}}
					/>
					<button type="button" class="select" onclick={() => onSelect(entry.snowflake)}>
						<span class="name" title={label(entry)}>{label(entry)}</span>
						{#if invalid}<span class="badge" title="Unsaved — invalid">!</span>{/if}
					</button>
					<button
						type="button"
						class="delete"
						aria-label="Delete entry"
						onclick={(e) => {
							e.stopPropagation();
							onDelete(entry.snowflake);
						}}>×</button
					>
				</li>
			{/each}
		</ul>

		{#if pageCount > 1}
			<div class="pager">
				<button type="button" disabled={safePage === 0} onclick={() => (page = safePage - 1)}
					>‹</button
				>
				<span>{safePage + 1} / {pageCount}</span>
				<button
					type="button"
					disabled={safePage >= pageCount - 1}
					onclick={() => (page = safePage + 1)}>›</button
				>
			</div>
		{/if}
	{/if}
</div>

<style>
	.entry-list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		min-height: 0;
		flex: 1 1 auto;
	}
	.controls {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: 0 var(--spacing-sm);
	}
	.sort {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.rows {
		list-style: none;
		margin: 0;
		padding: 0 var(--spacing-sm);
		overflow-y: auto;
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm, 4px);
		color: var(--color-text-secondary);
	}
	.row.selected {
		background: var(--color-bg-selected, var(--color-border-subtle));
		color: var(--color-text-primary);
	}
	.row.disabled .name {
		opacity: 0.55;
	}
	.toggle {
		flex: 0 0 auto;
	}
	.select {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		text-align: left;
		background: none;
		border: none;
		padding: 0;
		cursor: pointer;
		color: inherit;
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.badge {
		flex: 0 0 auto;
		font-weight: bold;
		color: var(--color-error-text);
	}
	.delete {
		flex: 0 0 auto;
		line-height: 1;
		opacity: 0.6;
	}
	.delete:hover {
		opacity: 1;
	}
	.pager {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.empty {
		padding: var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
