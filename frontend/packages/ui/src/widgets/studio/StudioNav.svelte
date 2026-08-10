<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import type { StudioSection } from '$lib/navigation/routes.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';

	let { section }: { section: StudioSection } = $props();

	var selectedWorld = $state();

	const realms = $derived(realmStore.summaries);

	const activeRealmExists = $derived(selectedWorld !== 'None');

	const sections: Array<{ id: StudioSection; label: string; go: () => void }> = [
		{ id: 'realm', label: 'Realm', go: () => navigator.start().studio().realm() },
		{ id: 'characters', label: 'Characters', go: () => navigator.start().studio().characters() },
		{ id: 'lorebooks', label: 'Lorebooks', go: () => navigator.start().studio().lorebooks() },
		{ id: 'templates', label: 'Templates', go: () => navigator.start().studio().templates() },
		{ id: 'tools', label: 'Tools', go: () => navigator.start().studio().tools() },
		{ id: 'personas', label: 'Personas', go: () => navigator.start().studio().personas() },
		{ id: 'widgets', label: 'Widgets', go: () => navigator.start().studio().widgets() }
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
	{#each sections as { id, label, go } (id)}
		{#if id !== 'realm' || activeRealmExists}
			<button class="tab" class:active={section === id} onclick={go}>
				{label}
			</button>
		{/if}
	{/each}
</nav>

<style>
	.studioNav {
		display: flex;
		gap: 8px;
		padding: 8px 0;
	}
	.tab {
		padding: 6px 14px;
		border-radius: 999px;
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
	}
	.tab.active {
		border-color: currentColor;
	}
</style>
