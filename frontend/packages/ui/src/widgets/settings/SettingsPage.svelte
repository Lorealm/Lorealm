<script lang="ts">
	import { onMount } from 'svelte';
	import { configStore, SettingsSection } from '$lib/stores/serverConfig.svelte';
	import SettingsNav from './SettingsNav.svelte';
	import HostingSection from './sections/HostingSection.svelte';
	import StorageSection from './sections/StorageSection.svelte';
	import SessionsSection from './sections/SessionsSection.svelte';
	import UsersSection from './sections/UsersSection.svelte';
	import CharactersSection from './sections/CharactersSection.svelte';
	import TokenizersSection from './sections/TokenizersSection.svelte';
	import TasksSection from './sections/TasksSection.svelte';

	onMount(() => {
		if (!configStore.loaded) configStore.load();
	});
</script>

<div class="settings">
	{#if !configStore.loaded}
		<div class="status">Loading settings…</div>
	{:else}
		<SettingsNav />

		{#if configStore.section === SettingsSection.Hosting}
			<HostingSection />
		{:else if configStore.section === SettingsSection.Storage}
			<StorageSection />
		{:else if configStore.section === SettingsSection.Sessions}
			<SessionsSection />
		{:else if configStore.section === SettingsSection.Users}
			<UsersSection />
		{:else if configStore.section === SettingsSection.Characters}
			<CharactersSection />
		{:else if configStore.section === SettingsSection.Tokenizers}
			<TokenizersSection />
		{:else if configStore.section === SettingsSection.Tasks}
			<TasksSection />
		{/if}
	{/if}
</div>

<style>
	.settings {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		padding: var(--spacing-sm);
	}
	.status {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
	}
</style>
