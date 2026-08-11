<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import { router } from '$lib/navigation/router.svelte.js';
	import { configStore } from '$lib/stores/serverConfig.svelte';
	import UserSettings from './UserSettings.svelte';
	import ServerSettings from './ServerSettings.svelte';
	import ClientSettings from './ClientSettings.svelte';

	const route = $derived(router.current);
	const settings = $derived(route.view === 'settings' ? route : null);
	const scope = $derived(settings?.scope ?? 'user');
	const clientSection = $derived(settings?.scope === 'client' ? settings.section : 'editor');
	const userSection = $derived(settings?.scope === 'user' ? settings.section : 'tokenizers');
	const serverSection = $derived(settings?.scope === 'server' ? settings.section : 'hosting');
</script>

<div class="settings">
	<nav class="scopeNav">
		<button
			class="tab"
			class:active={scope === 'client'}
			onclick={() => navigator.start().settings().client()}
		>
			Client
		</button>
		<button
			class="tab"
			class:active={scope === 'user'}
			onclick={() => navigator.start().settings().user()}
		>
			User
		</button>
		{#if configStore.canEditServer}
			<button
				class="tab"
				class:active={scope === 'server'}
				onclick={() => navigator.start().settings().server()}
			>
				Server
			</button>
		{/if}
	</nav>

	{#if scope === 'client'}
		<ClientSettings section={clientSection} />
	{:else if scope === 'server' && configStore.canEditServer}
		<ServerSettings section={serverSection} />
	{:else}
		<UserSettings section={userSection} />
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
	.scopeNav {
		display: flex;
		gap: var(--spacing-sm);
		flex-shrink: 0;
		padding-bottom: var(--spacing-sm);
		margin-bottom: var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
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
</style>
