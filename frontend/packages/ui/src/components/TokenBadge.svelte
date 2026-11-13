<script lang="ts">
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import { countTokens, type TokenCount } from '#lib/stores/tokenCount.js';
	import { tokenizerStore } from '#lib/stores/tokenizer.svelte.js';

	let {
		text = '',
		debounceMs = 300
	}: {
		text?: string;
		debounceMs?: number;
	} = $props();

	let count = $state<TokenCount | null>(null);

	$effect(() => {
		const value = text;
		void tokenizerStore.activeId; // recount when the author swaps tokenizers

		if (value === '') {
			count = { tokens: 0, estimated: false };
			return;
		}

		let cancelled = false;
		const timer = setTimeout(async () => {
			const next = await countTokens(value);
			if (!cancelled) count = next;
		}, debounceMs);

		return () => {
			cancelled = true;
			clearTimeout(timer);
		};
	});

	const title = $derived(
		count?.estimated
			? 'Estimated — no tokenizer is active. Pick one in Settings for exact counts.'
			: 'Tokens under the active tokenizer'
	);
</script>

<span class="tokenBadge" class:estimated={count?.estimated} {@attach tooltip(title)}>
	{#if count === null}
		…
	{:else}
		{count.estimated ? '~' : ''}{count.tokens.toLocaleString()}
	{/if}
	tokens
</span>

<style>
	.tokenBadge {
		flex: 0 0 auto;
		padding: 0 var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-family: var(--font-family-body);
		font-size: var(--font-size-xs);
		line-height: 1.6;
		white-space: nowrap;
		font-variant-numeric: tabular-nums;
	}
	.tokenBadge.estimated {
		color: var(--color-text-muted);
		font-style: italic;
	}
</style>
