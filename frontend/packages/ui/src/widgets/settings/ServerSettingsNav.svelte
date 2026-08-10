<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import type { ServerSection } from '$lib/navigation/routes.js';
	import { configStore } from '$lib/stores/serverConfig.svelte';

	let { section }: { section: ServerSection } = $props();

	const server = () => navigator.start().settings().server();

	const sections: Array<{ id: ServerSection; label: string; go: () => void }> = [
		{ id: 'hosting', label: 'Hosting', go: () => server().hosting() },
		{ id: 'storage', label: 'Storage', go: () => server().storage() },
		{ id: 'sessions', label: 'Sessions', go: () => server().sessions() },
		{ id: 'users', label: 'Users', go: () => server().users() },
		{ id: 'characters', label: 'Characters', go: () => server().characters() },
		{ id: 'tokenizers', label: 'Tokenizers', go: () => server().tokenizers() },
		{ id: 'tasks', label: 'Tasks', go: () => server().tasks() }
	];
</script>

<nav class="settingsNav">
	{#each sections as { id, label, go } (id)}
		<button class="tab" class:active={section === id} onclick={go}>
			{label}
		</button>
	{/each}
</nav>

{#if !configStore.canEditServer}
	<p class="readonly">Read-only — you don't have the EDIT_SERVER permission.</p>
{/if}

<style>
	.settingsNav {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) 0;
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
	.readonly {
		margin: 0 0 var(--spacing-sm);
		font-size: var(--font-size-sm);
		color: var(--color-warning-text);
	}
</style>
