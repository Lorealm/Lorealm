<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { ArrowUpDown, Plus, Search } from '@lucide/svelte';
	import type { Snippet } from 'svelte';

	let {
		title,
		query = $bindable(''),
		sort = $bindable(''),
		sortOptions = [],
		searchPlaceholder,
		createLabel,
		oncreate,
		creating = false,
		layout = 'grid',
		empty,
		children
	}: {
		title: string;
		query?: string;
		sort?: string;
		sortOptions?: Array<{ value: string; label: string }>;
		searchPlaceholder?: string;
		createLabel: string;
		oncreate: () => void;
		creating?: boolean;
		/** `grid` tiles cards; `list` stacks full-width rows. */
		layout?: 'grid' | 'list';
		empty?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="page">
	<div class="topbar">
		<h2>{title}</h2>

		<div class="search">
			<Search size={14} />
			<input
				type="text"
				placeholder={searchPlaceholder ?? t.studio.list.search}
				bind:value={query}
				aria-label={searchPlaceholder ?? t.studio.list.search}
			/>
		</div>

		{#if sortOptions.length > 0}
			<div class="sort">
				<ArrowUpDown size={14} />
				<select bind:value={sort} aria-label={t.studio.list.sort(title)}>
					{#each sortOptions as option (option.value)}
						<option value={option.value}>{option.label}</option>
					{/each}
				</select>
			</div>
		{/if}

		<button type="button" class="create" onclick={oncreate} disabled={creating}>
			<Plus size={16} />
			{creating ? t.studio.list.creating : createLabel}
		</button>
	</div>

	<div class="body">
		{#if empty}
			{@render empty()}
		{/if}
		<div class="items" class:list={layout === 'list'}>
			{@render children()}
		</div>
	</div>
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
	}

	.topbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-md) var(--spacing-xl);
		border-bottom: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}

	h2 {
		flex: 0 0 auto;
		margin: 0;
		font-size: var(--font-size-h4);
	}

	.search {
		flex: 1 1 auto;
		min-width: 0;
		max-width: 320px;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-sm);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated);
		color: var(--color-text-muted);
	}
	.search input {
		background: transparent;
		padding: var(--spacing-xs) 0;
	}

	.sort {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-secondary);
	}

	.sort select {
		width: auto;
		min-width: 150px;
	}

	.create {
		flex: 0 0 auto;
		margin-left: auto;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	.create:hover:not(:disabled) {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}
	.create:disabled {
		opacity: 0.6;
	}

	.body {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-xl);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
	}

	.items {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
		gap: var(--grid-gap);
		align-content: start;
	}

	.items.list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	@media (max-width: 768px) {
		.topbar {
			flex-wrap: wrap;
			gap: var(--spacing-sm);
			padding: var(--spacing-md);
		}
		h2 {
			flex: 1 1 auto;
		}
		.search,
		.sort {
			order: 1;
		}
		.search {
			flex: 1 1 8em;
			max-width: none;
		}
		.sort {
			flex: 1 1 8em;
			min-width: 0;
		}
		.sort select {
			flex: 1 1 auto;
			min-width: 0;
		}
		.body {
			padding: var(--spacing-md);
		}
		.items {
			grid-template-columns: 1fr;
		}
	}
</style>
