<script lang="ts">
	import { configStore } from '$lib/stores/serverConfig.svelte';
	import { settingsNav, ServerSection } from '$lib/stores/settings.svelte';

	const sections = [
		{ id: ServerSection.Hosting, label: 'Hosting' },
		{ id: ServerSection.Storage, label: 'Storage' },
		{ id: ServerSection.Sessions, label: 'Sessions' },
		{ id: ServerSection.Users, label: 'Users' },
		{ id: ServerSection.Characters, label: 'Characters' },
		{ id: ServerSection.Tokenizers, label: 'Tokenizers' },
		{ id: ServerSection.Tasks, label: 'Tasks' }
	];
</script>

<nav class="settingsNav">
	{#each sections as { id, label } (id)}
		<button
			class="tab"
			class:active={settingsNav.serverSection === id}
			onclick={() => settingsNav.setServerSection(id)}
		>
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
