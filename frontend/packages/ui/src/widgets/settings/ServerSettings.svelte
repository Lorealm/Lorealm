<script lang="ts">
	import { configStore } from '$lib/stores/serverConfig.svelte';
	import type { ServerSection } from '$lib/navigation/routes.js';
	import ServerSettingsNav from './ServerSettingsNav.svelte';
	import HostingSection from './sections/HostingSection.svelte';
	import StorageSection from './sections/StorageSection.svelte';
	import SessionsSection from './sections/SessionsSection.svelte';
	import UsersSection from './sections/UsersSection.svelte';
	import RolesSection from './sections/RolesSection.svelte';
	import CharactersSection from './sections/CharactersSection.svelte';
	import TokenizersSection from './sections/TokenizersSection.svelte';
	import TasksSection from './sections/TasksSection.svelte';

	let { section }: { section: ServerSection } = $props();
</script>

<div class="server">
	{#if !configStore.loaded}
		<div class="status">Loading settings…</div>
	{:else}
		<ServerSettingsNav {section} />

		{#if section === 'hosting'}
			<HostingSection />
		{:else if section === 'storage'}
			<StorageSection />
		{:else if section === 'sessions'}
			<SessionsSection />
		{:else if section === 'users'}
			<UsersSection />
		{:else if section === 'roles'}
			<RolesSection />
		{:else if section === 'characters'}
			<CharactersSection />
		{:else if section === 'tokenizers'}
			<TokenizersSection />
		{:else if section === 'tasks'}
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
