<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import UserSettings from './UserSettings.svelte';
	import ServerSettings from './ServerSettings.svelte';

	const route = $derived(router.current);
	const settings = $derived(route.view === 'settings' ? route : null);
	const scope = $derived(settings?.scope ?? 'user');
	const userSection = $derived(settings?.scope === 'user' ? settings.section : 'profile');
	const serverSection = $derived(settings?.scope === 'server' ? settings.section : 'hosting');

	onMount(() => {
		configStore.loadPermissions();
	});
</script>

<div class="settings">
	{#if configStore.canSeeServer}
		<nav class="scopeNav">
			<button
				class="tab"
				class:active={scope === 'user'}
				onclick={() => navigator.start().settings().user()}
			>
				{t.nav.scopes.user}
			</button>
			<button
				class="tab"
				class:active={scope === 'server'}
				onclick={() => navigator.start().settings().server()}
			>
				{t.nav.scopes.server}
			</button>
		</nav>
	{/if}

	{#if scope === 'server' && configStore.canSeeServer}
		<ServerSettings section={serverSection} />
	{:else}
		<UserSettings section={userSection} id={settings?.scope === 'user' ? settings.id : undefined} />
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
