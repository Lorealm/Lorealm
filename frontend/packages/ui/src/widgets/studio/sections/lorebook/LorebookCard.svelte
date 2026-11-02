<script lang="ts">
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { navigator } from '$lib/navigation/navigator.js';
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import type { LorebookMetadata } from '$lib/types/lorebook.js';
	import { BookOpen, Check, Pencil, Trash2 } from '@lucide/svelte';

	let { meta }: { meta: LorebookMetadata } = $props();

	const active = $derived(lorebookStore.isActive(meta.id));

	let confirmingDelete = $state(false);
	let busy = $state(false);

	async function toggleActive() {
		if (busy) return;
		busy = true;
		try {
			await lorebookStore.setActive(meta.id);
		} finally {
			busy = false;
		}
	}

	async function remove() {
		confirmingDelete = false;
		busy = true;
		try {
			await lorebookStore.deleteLorebook(meta.id);
		} catch (e) {
			console.error(e);
		} finally {
			busy = false;
		}
	}
</script>

<article class="card" class:active>
	<header class="head">
		<h3 class="name" {@attach tooltip(meta.name)}>{meta.name}</h3>
		{#if active}
			<span class="pill">Active</span>
		{/if}
	</header>

	<p class="description" class:none={!meta.description}>
		{meta.description || 'No description'}
	</p>

	<div class="meta">
		<span class="stat" {@attach tooltip('Entries in this lorebook')}>
			<BookOpen size={13} />
			{meta.entry_count}
			{meta.entry_count === 1 ? 'entry' : 'entries'}
		</span>
	</div>

	{#if confirmingDelete}
		<div class="controls confirm">
			<span class="confirmText">Delete this lorebook?</span>
			<button type="button" class="danger" onclick={remove}>Delete</button>
			<button type="button" onclick={() => (confirmingDelete = false)}>Cancel</button>
		</div>
	{:else}
		<div class="controls">
			<button type="button" class="toggle" class:on={active} disabled={busy} onclick={toggleActive}>
				<Check size={14} />
				{active ? 'Deactivate' : 'Activate'}
			</button>
			<button type="button" onclick={() => navigator.start().studio().lorebooks(meta.id)}>
				<Pencil size={14} />
				Edit
			</button>
			<button
				type="button"
				class="del"
				{@attach tooltip('Delete lorebook')}
				aria-label={`Delete ${meta.name}`}
				disabled={busy}
				onclick={() => (confirmingDelete = true)}
			>
				<Trash2 size={14} />
			</button>
		</div>
	{/if}
</article>

<style>
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		min-height: 150px;
		padding: var(--spacing-lg);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		background-color: var(--color-bg-elevated);
	}
	.card:hover {
		border-color: var(--color-border-medium);
		box-shadow: var(--shadow-md);
	}
	.card.active {
		border-color: var(--color-success-border);
	}

	.head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.name {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-semibold);
		text-transform: none;
		letter-spacing: normal;
	}
	.pill {
		flex: 0 0 auto;
		padding: 0 var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--color-success-bg);
		color: var(--color-success-text);
		font-size: var(--font-size-xs);
		line-height: 1.6;
	}

	.description {
		flex: 1 1 auto;
		margin: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-snug);
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		overflow: hidden;
	}
	.description.none {
		color: var(--color-text-muted);
		font-style: italic;
	}

	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-md);
	}
	.stat {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.controls button {
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.controls button:disabled {
		opacity: 0.5;
	}
	.toggle {
		flex: 1 1 auto;
	}
	.toggle.on {
		background-color: var(--color-success-bg);
		color: var(--color-success-text);
	}
	.del {
		flex: 0 0 auto;
		background-color: transparent;
		color: var(--color-text-muted);
	}
	.del:hover {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}

	.confirm {
		flex-wrap: wrap;
	}
	.confirmText {
		flex: 1 1 100%;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.danger {
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.danger:hover {
		background-color: var(--color-error);
		color: var(--color-text-white);
	}
</style>
