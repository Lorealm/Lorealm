<script lang="ts">
	import { onMount } from 'svelte';
	import { configStore } from '$lib/stores/serverConfig.svelte';
	import { settingsNav, ServerSection } from '$lib/stores/settings.svelte';
	import ServerSettingsNav from './ServerSettingsNav.svelte';
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

<div class="server">
	{#if !configStore.loaded}
		<div class="status">Loading settings…</div>
	{:else}
		<ServerSettingsNav />

		{#if settingsNav.serverSection === ServerSection.Hosting}
			<HostingSection />
		{:else if settingsNav.serverSection === ServerSection.Storage}
			<StorageSection />
		{:else if settingsNav.serverSection === ServerSection.Sessions}
			<SessionsSection />
		{:else if settingsNav.serverSection === ServerSection.Users}
			<UsersSection />
		{:else if settingsNav.serverSection === ServerSection.Characters}
			<CharactersSection />
		{:else if settingsNav.serverSection === ServerSection.Tokenizers}
			<TokenizersSection />
		{:else if settingsNav.serverSection === ServerSection.Tasks}
			<TasksSection />
		{/if}
	{/if}
</div>

<style>
	.server {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}
	.status {
		display: flex;
		flex: 1;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
	}
</style>
