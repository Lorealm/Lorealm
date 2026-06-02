<script>
	import { navigationStore, SidebarView } from '$lib/stores/navigation.svelte';
	import ChatBox from '$lib/widgets/chat/ChatBox.svelte';
	import CharactersPage from '../characters/CharactersPage.svelte';
	import ChatControls from '../chat/ChatControls.svelte';
	import Placeholder from '../Placeholder.svelte';
	import SettingsPage from '../settings/SettingsPage.svelte';
	import StudioRouter from './StudioRouter.svelte';
</script>

{#if navigationStore.sidebarActive === SidebarView.Write}
	<ChatBox />
	<ChatControls />
{:else if navigationStore.sidebarActive === SidebarView.Characters}
	<CharactersPage />
{:else if navigationStore.sidebarActive === SidebarView.Realm}
	<Placeholder label="Realms" />
{:else if navigationStore.sidebarActive === SidebarView.Studio}
	<StudioRouter />
{:else if navigationStore.sidebarActive === SidebarView.Settings}
	<SettingsPage />
{/if}
