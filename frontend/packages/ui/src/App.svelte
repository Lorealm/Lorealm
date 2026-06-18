<script lang="ts">
	import '$lib/assets/css/variables.css';
	import '$lib/assets/css/standardControls.css';

	import type { PlatformAPI } from './index.ts';
	import { setAPI } from './core/context.ts';
	import Router from './Router.svelte';
	import { createApiClient } from './api/client.ts';
	import { setAuthState } from './stores/auth.svelte.ts';
	import { ActiveView, navigationStore } from './stores/navigation.svelte.ts';
	import Titlebar from './widgets/Titlebar.svelte';

	let { api }: { api: PlatformAPI } = $props();

	// svelte-ignore state_referenced_locally
	setAPI(api);
	// svelte-ignore state_referenced_locally
	createApiClient(api);

	$effect(() => {
		api.auth
			.hydrate()
			.then((session) => {
				if (session) {
					console.log('Valid session!');
					setAuthState({ status: 'authenticated', session });
					navigationStore.setView(ActiveView.Main);
				} else {
					setAuthState({ status: 'unauthenticated' });
					navigationStore.setView(ActiveView.Login);
				}
			})
			.catch((error: Error) => {});
	});
</script>

<div class="app">
	<Titlebar {api} />
	<div class="app-body">
		<Router />
	</div>
</div>

<style>
	.app {
		display: flex;
		flex-direction: column;
		height: 100vh;
		background-color: var(--color-bg-base);
	}
	.app-body {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
