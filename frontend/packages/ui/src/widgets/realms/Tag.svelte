<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		selected = false,
		onclick,
		children
	}: { selected?: boolean; onclick?: () => void; children: Snippet } = $props();
</script>

{#if onclick}
	<button type="button" class="tag" class:selected onclick={() => onclick?.()}>
		{@render children()}
	</button>
{:else}
	<span class="tag">{@render children()}</span>
{/if}

<style>
	.tag {
		display: inline-flex;
		align-items: center;
		font-size: var(--font-size-sm);
		line-height: 1;
		padding: 4px 10px;
		border-radius: var(--radius-full);
		border: 1px solid var(--color-border-subtle);
		background: transparent;
		color: var(--color-text-secondary);
		white-space: nowrap;
	}

	button.tag {
		cursor: pointer;
	}

	button.tag:hover {
		border-color: var(--color-border-medium);
		color: var(--color-text-primary);
	}

	.selected {
		border-color: var(--color-border-active);
		background: var(--color-selected);
		color: var(--color-text-primary);
	}
</style>
