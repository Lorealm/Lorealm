<script lang="ts">
	import { getAuthState } from './stores/auth.svelte.ts';
	import Login from './views/Login.svelte';
	import Main from './views/Main.svelte';
</script>

{#if getAuthState().status === 'authenticated'}
	<Main />
{:else}
	<Login />
{/if}
