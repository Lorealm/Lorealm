<script lang="ts">
	import type { Suggestion } from '#lib/commands/index.js';

	let {
		items,
		activeIndex,
		heading,
		onSelect
	}: {
		items: Suggestion[];
		activeIndex: number;
		/** Names what is being offered when it is not commands. */
		heading?: string;
		onSelect: (suggestion: Suggestion) => void;
	} = $props();
</script>

<div class="suggestions" role="listbox" aria-label={heading ?? 'Commands'}>
	{#if heading}
		<p class="heading">{heading}</p>
	{/if}
	{#each items as suggestion, i (suggestion.key)}
		<button
			class="entry"
			class:active={i === activeIndex}
			role="option"
			aria-selected={i === activeIndex}
			tabindex="-1"
			onmousedown={(e) => {
				e.preventDefault(); // keep focus in the composer
				onSelect(suggestion);
			}}
		>
			<span class="primary">{suggestion.primary}</span>
			<span class="secondary">{suggestion.secondary}</span>
		</button>
	{/each}
</div>

<style>
	.suggestions {
		position: absolute;
		bottom: calc(100% + 6px);
		left: 0;
		right: 0;
		z-index: 50;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		max-height: 15em;
		overflow-y: auto;
		padding: var(--spacing-xs);
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
	}
	.heading {
		margin: 0;
		padding: var(--spacing-xs) var(--spacing-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm, 0.85rem);
	}
	.entry {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
		width: 100%;
		padding: var(--spacing-sm);
		border: none;
		border-radius: var(--radius-sm, 6px);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.entry:hover,
	.entry.active {
		background-color: var(--color-bg-base);
	}
	.primary {
		color: var(--color-text-accent);
		font-family: var(--font-mono, monospace);
		font-size: var(--font-size-sm, 0.85rem);
	}
	.secondary {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm, 0.85rem);
	}
</style>
