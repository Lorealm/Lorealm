<script lang="ts">
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { ChevronDown } from '@lucide/svelte';

	let activeCharacter = $derived(chatStore.characterId);
	let imageEndpoint = $derived(`/api/v1/characters/${activeCharacter}/image`);
	let collapsed = $state(false);
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">Character Image</span>
	</button>
	{#if !collapsed}
		{#if activeCharacter}
			<TransportImage src={imageEndpoint} />
		{:else}
			<div class="empty">No active character.</div>
		{/if}
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-large, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.head {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		text-align: left;
		cursor: pointer;
	}
	.label {
		flex: 1;
		min-width: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev :global(svg) {
		width: 14px;
		height: 14px;
	}
	.empty {
		margin: auto;
		padding: var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-align: center;
	}
	.widget :global(.transport-image) {
		flex: 1 1 auto;
		min-height: 0;
		width: 100%;
		overflow: hidden;
	}
</style>
