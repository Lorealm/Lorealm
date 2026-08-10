<script lang="ts">
	import { ArrowUpDown, Plus, Search } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		title,
		createLabel = 'New',
		oncreate,
		creating = false,
		query = $bindable(''),
		sort = $bindable(''),
		sortOptions = [],
		searchPlaceholder = 'Search…',
		status,
		children
	}: {
		title: string;
		createLabel?: string;
		oncreate: () => void;
		creating?: boolean;
		query?: string;
		sort?: string;
		sortOptions?: Array<{ value: string; label: string }>;
		searchPlaceholder?: string;
		/** Save indicator and the like, under the filters. */
		status?: Snippet;
		/** The list rows. Styled by the caller, which declares them. */
		children: Snippet;
	} = $props();
</script>

<section class="col-list">
	<header class="list-header">
		<div class="title-row">
			<span class="title">{title}</span>
			<button type="button" class="new" onclick={oncreate} disabled={creating}>
				<Plus size={14} />
				{creating ? 'Creating…' : createLabel}
			</button>
		</div>

		<div class="filters">
			<div class="search">
				<Search size={13} />
				<input
					type="text"
					placeholder={searchPlaceholder}
					bind:value={query}
					aria-label={searchPlaceholder}
				/>
			</div>
			{#if sortOptions.length > 0}
				<div class="sort">
					<ArrowUpDown size={13} />
					<select bind:value={sort} aria-label={`Sort ${title.toLowerCase()}`}>
						{#each sortOptions as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				</div>
			{/if}
		</div>

		{@render status?.()}
	</header>

	<nav class="list">
		{@render children()}
	</nav>
</section>

<style>
	.col-list {
		flex: 0 0 240px;
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
		border-right: 1px solid var(--color-border-subtle);
	}
	.list-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.title-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.title {
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.new {
		flex: 0 0 auto;
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.new:disabled {
		opacity: 0.6;
	}

	/* stacked rather than side by side: 240px doesn't fit both legibly */
	.filters {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.search,
	.sort {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated);
		color: var(--color-text-muted);
	}
	.search input {
		min-width: 0;
		padding: var(--spacing-xs) 0;
		background: transparent;
		font-size: var(--font-size-sm);
	}
	.sort {
		padding-right: 0;
	}
	/* Undo the global `select { width: 100% }` so the icon keeps its place */
	.sort select {
		flex: 1 1 auto;
		min-width: 0;
		width: auto;
		background-color: transparent;
		font-size: var(--font-size-sm);
	}

	.list {
		display: flex;
		flex-direction: column;
		padding: var(--spacing-sm);
		gap: var(--spacing-xs);
		overflow-y: auto;
	}
</style>
