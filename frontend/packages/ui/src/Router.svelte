<!-- src/lib/Router.svelte -->
<script lang="ts">
	import Login from './views/Login.svelte';

	// simple reactive state, or use a small store
	let current = $state<'login' | 'settings'>('login');

	export function navigate(route: typeof current) {
		current = route;
	}
</script>

{#if current === 'login'}
	<Login />
{/if}
