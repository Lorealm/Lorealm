<script>
	import { chatStore } from '$lib/stores/chat.svelte';
	import { navigationStore, SidebarView } from '$lib/stores/navigation.svelte';
	import ChatBox from '$lib/widgets/chat/ChatBox.svelte';
	import CharactersPage from '../characters/CharactersPage.svelte';
	import ChatControls from '../chat/ChatControls.svelte';
	import RealmChatControls from '../chat/RealmChatControls.svelte';
	import ConnectionsView from '../connections/ConnectionsView.svelte';
	import RealmsPage from '../realms/RealmsPage.svelte';
	import SettingsPage from '../settings/SettingsPage.svelte';
	import StudioRouter from './StudioRouter.svelte';
</script>

{#if navigationStore.sidebarActive === SidebarView.Write}
	<ChatBox />
	{#if chatStore.isRealm}
		<RealmChatControls />
	{:else}
		<ChatControls />
	{/if}
{:else if navigationStore.sidebarActive === SidebarView.Characters}
	<CharactersPage />
{:else if navigationStore.sidebarActive === SidebarView.Realm}
	<RealmsPage />
{:else if navigationStore.sidebarActive === SidebarView.Connections}
	<ConnectionsView />
{:else if navigationStore.sidebarActive === SidebarView.Studio}
	<StudioRouter />
{:else if navigationStore.sidebarActive === SidebarView.Settings}
	<SettingsPage />
{/if}
