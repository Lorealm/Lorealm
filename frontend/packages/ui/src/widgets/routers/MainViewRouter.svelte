<script lang="ts">
	import { router } from '$lib/navigation/router.svelte.js';
	import ChatBox from '$lib/widgets/chat/ChatBox.svelte';
	import CharactersPage from '../characters/CharactersPage.svelte';
	import ChatControls from '../chat/ChatControls.svelte';
	import ConnectionsView from '../connections/ConnectionsView.svelte';
	import RealmsPage from '../realms/RealmsPage.svelte';
	import SettingsPage from '../settings/SettingsPage.svelte';
	import StudioRouter from './StudioRouter.svelte';

	const route = $derived(router.current);
</script>

{#if router.status === 'error'}
	<div class="loadError" role="alert">
		<span>{router.error ?? 'Something failed to load.'}</span>
		<button type="button" onclick={() => router.enter()}>Retry</button>
	</div>
{/if}

{#if route.view === 'write'}
	<ChatBox />
	<ChatControls />
{:else if route.view === 'characters'}
	<CharactersPage />
{:else if route.view === 'realms'}
	<RealmsPage />
{:else if route.view === 'connections'}
	<ConnectionsView />
{:else if route.view === 'studio'}
	<StudioRouter section={route.section} id={route.id} />
{:else if route.view === 'settings'}
	<SettingsPage />
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
