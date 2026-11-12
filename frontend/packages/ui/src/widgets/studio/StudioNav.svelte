<script lang="ts">
	import { STUDIO_LABEL } from '$lib/navigation/breadcrumbs.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import type { StudioSection } from '$lib/navigation/routes.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { importFiles } from '$lib/stores/transfer.js';
	import { Upload } from '@lucide/svelte';

	let { section }: { section: StudioSection } = $props();

	var selectedWorld = $state();

	const realms = $derived(realmStore.summaries);

	const activeRealmExists = $derived(selectedWorld !== 'None');

	let inputEl: HTMLInputElement;

	function onPick(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = Array.from(input.files ?? []);
		input.value = '';
		if (files.length > 0) void importFiles(files);
	}

	const sections: Array<{ id: StudioSection; go: () => void }> = [
		{ id: 'realm', go: () => navigator.start().studio().realm() },
		{ id: 'characters', go: () => navigator.start().studio().characters() },
		{ id: 'lorebooks', go: () => navigator.start().studio().lorebooks() },
		{ id: 'templates', go: () => navigator.start().studio().templates() },
		{ id: 'tools', go: () => navigator.start().studio().tools() },
		{ id: 'modules', go: () => navigator.start().studio().modules() },
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
	<input
		bind:this={inputEl}
		type="file"
		multiple
		accept="image/png,.png,application/json,.json"
		onchange={onPick}
		hidden
		style="display:none"
	/>
	<button class="tab import" onclick={() => inputEl.click()}>
		<Upload size={16} />
		Import
	</button>
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
	.import {
		margin-left: auto;
	}
	.tab.active {
		border-color: currentColor;
	}

	@media (max-width: 768px) {
		.studioNav {
			padding: var(--spacing-sm);
			overflow-x: auto;
			scrollbar-width: none;
		}
		.studioNav > * {
			flex-shrink: 0;
			white-space: nowrap;
		}
		.studioNav select {
			width: auto;
			max-width: 9em;
		}
	}
</style>
