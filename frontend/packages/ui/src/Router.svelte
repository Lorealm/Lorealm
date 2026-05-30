<script lang="ts">
	import { ActiveView, navigationStore } from './stores/navigation.svelte.ts';
	import Login from './views/Login.svelte';
	import Main from './views/Main.svelte';
</script>

{#if navigationStore.active === ActiveView.Login}
	<Login />
{:else if navigationStore.active === ActiveView.Main}
	<Main />
{/if}
