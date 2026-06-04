<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { Check, Pencil } from '@lucide/svelte';

	let { id }: { id: Snowflake } = $props();

	const entry = $derived(lorebookStore.metadataList?.find((x) => x.id == id) ?? null);
	const active = $derived(lorebookStore.isActive(entry?.id));
</script>

<div class="card" class:active>
	<div class="header" class:active>{entry?.name ?? 'Loading...'}</div>
	<div class="description" class:active>{entry?.description ?? ''}</div>
	<div class="controls" class:active>
		<button
			class="btn-primary"
			disabled={!entry || active}
			onclick={() => {
				lorebookStore.setActive(entry!.id);
			}}><Check />Select</button
		>
		<button
			class="btn-ghost"
			disabled={!entry}
			onclick={() => {
				studioStore.open(entry!.id);
			}}><Pencil />Edit</button
		>
	</div>
</div>

<style>
	.card {
		display: flex;
		flex-direction: column;
		justify-content: center;
		width: 15em;
		min-height: 3em;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-elevated);
	}
	.card.active {
		background-color: var(--color-success-bg);
	}
	.header {
		color: var(--color-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-semibold);
	}
	.header.active {
		color: var(--color-success-text);
	}
	.description {
		color: var(--color-text-muted);
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
	}
	.description.active {
		color: var(--color-success);
	}
	.controls {
		display: flex;
		flex-direction: row;
		margin-top: var(--spacing-sm);
		gap: var(--spacing-sm);
	}

	.controls > button {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		transition:
			background-color 0.15s ease,
			color 0.15s ease;
	}

	.controls > button :global(svg) {
		width: 1.3em;
		height: 1.3em;
		flex-shrink: 0; /* prevents squishing in tight flex layouts */
	}

	.controls > button.btn-primary {
		background-color: var(--color-primary);
		color: var(--color-bg-elevated);
	}

	.controls.active > button.btn-primary {
		background-color: var(--color-success-bg);
		color: var(--color-success-text);
	}
	.controls.active > button.btn-primary:hover {
		background-color: var(--color-success);
	}

	.controls > button.btn-primary:hover {
		background-color: var(--color-primary-hover, var(--color-primary));
		color: var(--color-bg-elevated);
		filter: brightness(1.1);
	}

	.controls > button.btn-ghost {
		background-color: transparent;
	}
	.controls.active > button.btn-ghost:hover {
		color: var(--color-success);
	}
</style>
