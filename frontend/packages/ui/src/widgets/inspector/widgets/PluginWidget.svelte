<script lang="ts">
	import PluginSlotView from '#lib/components/PluginSlotView.svelte';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import type { PluginSlot } from '#lib/stores/plugins.svelte.js';
	import { ChevronDown } from '@lucide/svelte';

	let { slot }: { slot: PluginSlot } = $props();

	let collapsed = $state(false);
</script>

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">{slot.title}</span>
	</button>

	{#if !collapsed}
		<PluginSlotView {slot} conversation={chatStore.conversationId} fill />
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-height, 280px);
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
</style>
