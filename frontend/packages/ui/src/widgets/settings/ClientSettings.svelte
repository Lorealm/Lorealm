<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import type { ClientSection } from '$lib/navigation/routes.js';
	import EditorSection from './client/EditorSection.svelte';
	import InspectorSection from './client/InspectorSection.svelte';

	let { section }: { section: ClientSection } = $props();

	const sections: Array<{ id: ClientSection; label: string; go: () => void }> = [
		{
			id: 'editor',
			label: 'Editor',
			go: () => navigator.start().settings().client().editor()
		},
		{
			id: 'inspector',
			label: 'Inspector',
			go: () => navigator.start().settings().client().inspector()
		}
	];
</script>

<div class="client">
	<nav class="nav">
		{#each sections as { id, label, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>
				{label}
			</button>
		{/each}
	</nav>

	{#if section === 'editor'}
		<EditorSection />
	{:else if section === 'inspector'}
		<InspectorSection />
	{/if}
</div>

<style>
	.client {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}
	.nav {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) 0;
		flex-shrink: 0;
	}
	.tab {
		padding: 6px 14px;
		border-radius: var(--radius-full);
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
	}
	.tab.active {
		border-color: currentColor;
	}
</style>
