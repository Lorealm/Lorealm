<script lang="ts">
	import '$lib/assets/css/variables.css';
	import '$lib/assets/css/standardControls.css';

	import type { PlatformAPI } from './index.ts';
	import { setAPI } from './core/context.ts';
	import Router from './Router.svelte';
	import { createApiClient } from './api/client.ts';
	import { setAuthState } from './stores/auth.svelte.ts';
	import { ActiveView, navigationStore } from './stores/navigation.svelte.ts';

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

<Router />
