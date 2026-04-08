<script>
	import { navigationStore, SidebarView } from '$lib/stores/navigation.svelte';
	import ChatBox from '$lib/widgets/chat/ChatBox.svelte';
	import ChatControls from '../chat/ChatControls.svelte';
	import Placeholder from '../Placeholder.svelte';
</script>

{#if navigationStore.sidebarActive === SidebarView.Write}
	<ChatBox />
	<ChatControls />
{:else if navigationStore.sidebarActive === SidebarView.Characters}
	<Placeholder label="Characters" />
{:else if navigationStore.sidebarActive === SidebarView.Worlds}
	<Placeholder label="Worlds" />
{:else if navigationStore.sidebarActive === SidebarView.Studio}
	<Placeholder label="Studio" />
{:else if navigationStore.sidebarActive === SidebarView.Settings}
	<Placeholder label="Settings" />
{/if}
