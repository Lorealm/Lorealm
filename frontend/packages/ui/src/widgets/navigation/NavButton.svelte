<script lang="ts">
	import type { Component } from 'svelte';
	import { router } from '#lib/navigation/router.svelte.js';
	import type { ViewName } from '#lib/navigation/routes.js';

	let {
		label,
		Icon,
		collapsed,
		view,
		onselect
	}: {
		label: string;
		Icon: Component;
		collapsed: boolean;
		/** The view this button lights up for. */
		view: ViewName;
		onselect: () => void;
	} = $props();

	let active = $derived(router.current.view === view);
</script>

<button class="container" class:active onclick={onselect}>
	<Icon />
	{#if !collapsed}
		<span>{label}</span>
	{/if}
</button>

<style>
	.container {
		display: flex;
		flex-direction: row;
		width: 100%;
		align-items: center;
	}
	.container > span {
		font-size: var(--font-size-lg);
		margin-left: 0.5em;
	}
	.container.active {
		background-color: var(--color-active);
		color: var(--color-text-accent);
	}
	.container.active:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
</style>
