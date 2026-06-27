<script lang="ts">
	import type { Component } from 'svelte';
	import { navigationStore, type SidebarView } from '$lib/stores/navigation.svelte';

	let {
		label,
		Icon,
		collapsed,
		view
	}: { label: string; Icon: Component; collapsed: boolean; view: SidebarView } = $props();

	let active = $derived(navigationStore.sidebarActive === view);

	function handleClick() {
		navigationStore.setSidebarView(view);
	}
</script>

<button class="container" class:active onclick={handleClick}>
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
