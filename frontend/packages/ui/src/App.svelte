<script lang="ts">
	import '$lib/assets/css/variables.css';
	import '$lib/assets/css/standardControls.css';

	import type { PlatformAPI } from './index.ts';
	import { setAPI } from './core/context.ts';
	import Router from './Router.svelte';
	import { provideApiClient } from './api/client.ts';
	import { setAuthState } from './stores/auth.svelte.ts';
	import Titlebar from './widgets/Titlebar.svelte';
	import { showToast } from './stores/errorToast.svelte.ts';
	import { memory } from './stores/memory.svelte.ts';
	import { settings } from './stores/settings.svelte.ts';
	import { applyStyle, styles } from './stores/styles.svelte.ts';
	import { inspectorMatrix } from './stores/inspectorMatrix.svelte.ts';
	import { keybindConfig } from './keys/config.svelte.ts';

	let { api }: { api: PlatformAPI } = $props();

	// svelte-ignore state_referenced_locally
	setAPI(api);
	// The stores and the route loader reach the client through `useApi`, so it
	// has to exist before the first route is entered below.
	// svelte-ignore state_referenced_locally
	provideApiClient(api);

	$effect(() => {
		// Local config first: the login form reads it, and it needs no session.
		Promise.all([
			memory.load(),
			settings.load(),
			styles.load(),
			inspectorMatrix.load(),
			keybindConfig.load()
		])
			.then(() => api.auth.hydrate())
			.then((session) => {
				if (session) setAuthState({ status: 'authenticated', session });
				else setAuthState({ status: 'unauthenticated' });
			})
			.catch((error: Error) => {
				showToast({ message: error.message, variant: 'error' });
			});
	});

	$effect(() => {
		applyStyle(styles.tokens, styles.css);
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
