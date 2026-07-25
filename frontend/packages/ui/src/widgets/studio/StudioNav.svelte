<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { studioStore, StudioSection } from '$lib/stores/studio.svelte';

	var selectedWorld = $state();

	const realms = realmStore.summaries;

	const activeRealmExists = $derived(selectedWorld !== 'None');

	const sections = [
		{ id: StudioSection.Realm, label: 'Realm' },
		{ id: StudioSection.Characters, label: 'Characters' },
		{ id: StudioSection.Lorebooks, label: 'Lorebooks' },
		{ id: StudioSection.Templates, label: 'Templates' },
		{ id: StudioSection.Tools, label: 'Tools' },
		{ id: StudioSection.Personas, label: 'Personas' },
		{ id: StudioSection.Widgets, label: 'Widgets' }
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
	{#each sections as { id, label } (id)}
		{#if id !== StudioSection.Realm || activeRealmExists}
			<button
				class="tab"
				class:active={studioStore.section === id}
				onclick={() => studioStore.setSection(id)}
			>
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
