<script lang="ts">
	import { STUDIO_LABEL } from '$lib/navigation/breadcrumbs.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import type { StudioSection } from '$lib/navigation/routes.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';

	let { section }: { section: StudioSection } = $props();

	var selectedWorld = $state();

	const realms = $derived(realmStore.summaries);

	const activeRealmExists = $derived(selectedWorld !== 'None');

	const sections: Array<{ id: StudioSection; go: () => void }> = [
		{ id: 'realm', go: () => navigator.start().studio().realm() },
		{ id: 'characters', go: () => navigator.start().studio().characters() },
		{ id: 'lorebooks', go: () => navigator.start().studio().lorebooks() },
		{ id: 'templates', go: () => navigator.start().studio().templates() },
		{ id: 'tools', go: () => navigator.start().studio().tools() },
		{ id: 'personas', go: () => navigator.start().studio().personas() },
		{ id: 'widgets', go: () => navigator.start().studio().widgets() }
	];
</script>

<nav class="studioNav">
	<select bind:value={selectedWorld}>
		<option>None</option>
		<hr />
		{#each realms as realm (realm.id)}
			<option>{realm.name}</option>
		{/each}
	</select>
	<hr />
	{#each sections as { id, go } (id)}
		{#if id !== 'realm' || activeRealmExists}
			<button class="tab" class:active={section === id} onclick={go}>
				{STUDIO_LABEL[id]}
			</button>
		{/if}
	{/each}
</nav>

<style>
	.studioNav {
		display: flex;
		gap: var(--spacing-xs);
		padding: var(--spacing-md);
		background-color: var(--color-bg-panel);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.tab {
		justify-content: center;
		padding: var(--spacing-xs) var(--spacing-md);
		border-radius: var(--radius-full);
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
	}
	.tab.active {
		border-color: currentColor;
	}
</style>
