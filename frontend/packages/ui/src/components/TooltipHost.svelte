<script lang="ts">
	import { onMount } from 'svelte';
	import { placeFloating } from './floating.ts';
	import { installTooltips, tooltipState } from '../stores/tooltip.svelte.ts';

	onMount(installTooltips);

	const id = $props.id();
	const options = $derived(tooltipState.options);
	const isText = $derived(typeof options?.content === 'string');

	// Rich content can change size after it lands (images, async text), so this
	// re-places on resize rather than just once.
	$effect(() => {
		const { panel, anchor } = tooltipState;
		const side = options?.side ?? 'top';
		if (!panel || !anchor) return;
		const place = () =>
			placeFloating(panel, anchor.getBoundingClientRect(), { side, align: 'center', gap: 6 });
		place();
		const observer = new ResizeObserver(place);
		observer.observe(panel);
		return () => observer.disconnect();
	});

	// Text was announced up front. A snippet only exists once it's showing.
	$effect(() => {
		const { anchor } = tooltipState;
		if (!anchor || !options || isText || options.interactive || options.label) return;
		anchor.setAttribute('aria-describedby', id);
		return () => anchor.removeAttribute('aria-describedby');
	});
</script>

{#if options}
	<div
		{id}
		class="tooltip"
		class:text={isText}
		class:interactive={options.interactive}
		role={options.interactive ? 'group' : 'tooltip'}
		aria-label={options.interactive ? options.label : undefined}
		bind:this={tooltipState.panel}
	>
		{#if typeof options.content === 'string'}{options.content}{:else}{@render options.content()}{/if}
	</div>
{/if}

<style>
	.tooltip {
		position: fixed;
		top: 0;
		left: 0;
		z-index: var(--z-index-tooltip);
		box-sizing: border-box;
		width: max-content;
		max-width: min(22rem, calc(100vw - 16px));
		padding: var(--spacing-xs) var(--spacing-sm);
		overflow: auto;
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-md);
		background: var(--color-bg-elevated-2);
		box-shadow: var(--shadow-xl);
		color: var(--color-text-primary);
		font-family: var(--font-family-body);
		font-size: var(--font-size-sm);
		line-height: 1.4;
		overflow-wrap: anywhere;
		pointer-events: none;
		animation: tooltip-in var(--transition-fast) var(--easing-default);
	}
	.tooltip.text {
		white-space: pre-line;
	}
	.tooltip.interactive {
		pointer-events: auto;
	}
	@keyframes tooltip-in {
		from {
			opacity: 0;
		}
	}
</style>
