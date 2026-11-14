<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { ServerSection } from '#lib/navigation/routes.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';

	let { section }: { section: ServerSection } = $props();

	const server = () => navigator.start().settings().server();

	const sections: Array<{ id: ServerSection; go: () => void }> = [
		{ id: 'hosting', go: () => server().hosting() },
		{ id: 'storage', go: () => server().storage() },
		{ id: 'sessions', go: () => server().sessions() },
		{ id: 'users', go: () => server().users() },
		{ id: 'accounts', go: () => server().accounts() },
		{ id: 'roles', go: () => server().roles() },
		{ id: 'characters', go: () => server().characters() },
		{ id: 'tokenizers', go: () => server().tokenizers() },
		{ id: 'tasks', go: () => server().tasks() }
	];
</script>

<nav class="settingsNav">
	{#each sections as { id, go } (id)}
		<button class="tab" class:active={section === id} onclick={go}>
			{t.nav.server[id]}
		</button>
	{/each}
</nav>

{#if !configStore.canEditServer && section !== 'roles' && section !== 'accounts'}
	<p class="readonly">{t.settings.readOnly}</p>
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
