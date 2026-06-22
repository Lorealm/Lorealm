<script lang="ts">
	import { configStore, SettingsSection } from '$lib/stores/serverConfig.svelte.js';

	const sections = [
		{ id: SettingsSection.Hosting, label: 'Hosting' },
		{ id: SettingsSection.Storage, label: 'Storage' },
		{ id: SettingsSection.Sessions, label: 'Sessions' },
		{ id: SettingsSection.Users, label: 'Users' },
		{ id: SettingsSection.Characters, label: 'Characters' },
		{ id: SettingsSection.Tokenizers, label: 'Tokenizers' },
		{ id: SettingsSection.Tasks, label: 'Tasks' }
	];
</script>

<nav class="settingsNav">
	{#each sections as { id, label } (id)}
		<button
			class="tab"
			class:active={configStore.section === id}
			onclick={() => configStore.setSection(id)}
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
