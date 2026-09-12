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
	import { backdrop } from './stores/backdrop.svelte.ts';
	import TransportImage from './components/TransportImage.svelte';
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
			backdrop.load(),
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
	{#if backdrop.image}
		<div class="backdrop" aria-hidden="true">
			<TransportImage src={backdrop.image} alt="" style="display:block;width:100%;height:100%" />
		</div>
	{/if}

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
		/* Makes .app a stacking context, so the backdrop's negative z-index puts
		   it above this background but below everything in the app — no child
		   needs a z-index of its own. */
		isolation: isolate;
	}
	/*
	 * Fixed, so it holds still while content scrolls. There is no tint layer:
	 * `--color-bg-base` above is the tint, and a style that wants more of the
	 * picture through simply lowers its alpha.
	 */
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: -1;
		opacity: var(--backdrop-opacity);
		filter: var(--backdrop-image-filter);
		pointer-events: none;
	}
	.app-body {
		flex: 1 1 auto;
		min-height: 0;
	}
</style>
