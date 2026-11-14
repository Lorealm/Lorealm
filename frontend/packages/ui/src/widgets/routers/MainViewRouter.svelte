<script module lang="ts">
	const charactersPage = () => import('../characters/CharactersPage.svelte');
	const realmsPage = () => import('../realms/RealmsPage.svelte');
	const connectionsView = () => import('../connections/ConnectionsView.svelte');
	const settingsPage = () => import('../settings/SettingsPage.svelte');
	const pluginsPage = () => import('../plugins/PluginsPage.svelte');
	const pluginPage = () => import('../plugins/PluginPage.svelte');
	const listedPluginPage = () => import('../plugins/ListedPluginPage.svelte');
	const studioRouter = () => import('./StudioRouter.svelte');
</script>

<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { lazy } from '#lib/core/lazy.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import ChatBox from '#lib/widgets/chat/ChatBox.svelte';
	import ChatControls from '../chat/ChatControls.svelte';
	import PluginComposerSlots from '#lib/components/PluginComposerSlots.svelte';
	import TurnControls from '../chat/TurnControls.svelte';
	import { chatStore } from '#lib/stores/chat.svelte.js';

	const route = $derived(router.current);
</script>

{#if router.status === 'error'}
	<div class="loadError" role="alert">
		<span>{router.error ?? t.nav.loadFailed}</span>
		<button type="button" onclick={() => router.enter()}>{t.nav.retry}</button>
	</div>
{/if}

{#if route.view === 'write'}
	<ChatBox />
	{#if chatStore.isEnsemble}
		<TurnControls />
	{/if}
	<PluginComposerSlots />
	<ChatControls />
{:else if route.view === 'characters'}
	{#await lazy(charactersPage) then CharactersPage}
		<CharactersPage />
	{/await}
{:else if route.view === 'realms'}
	{#await lazy(realmsPage) then RealmsPage}
		<RealmsPage />
	{/await}
{:else if route.view === 'connections'}
	{#await lazy(connectionsView) then ConnectionsView}
		<ConnectionsView />
	{/await}
{:else if route.view === 'studio'}
	{#await lazy(studioRouter) then StudioRouter}
		<StudioRouter section={route.section} id={route.id} />
	{/await}
{:else if route.view === 'plugins' && route.tab === 'marketplace' && route.id !== undefined}
	{#await lazy(listedPluginPage) then ListedPluginPage}
		<ListedPluginPage handle={route.id} />
	{/await}
{:else if route.view === 'plugins' && route.id !== undefined}
	{#await lazy(pluginPage) then PluginPage}
		<PluginPage handle={route.id} />
	{/await}
{:else if route.view === 'plugins'}
	{#await lazy(pluginsPage) then PluginsPage}
		<PluginsPage tab={route.tab ?? 'installed'} />
	{/await}
{:else if route.view === 'settings'}
	{#await lazy(settingsPage) then SettingsPage}
		<SettingsPage />
	{/await}
{/if}

<style>
	.loadError {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
		flex-shrink: 0;
		padding: var(--spacing-sm) var(--spacing-lg);
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
	.loadError button {
		flex: 0 0 auto;
		padding: 2px 10px;
	}
</style>
