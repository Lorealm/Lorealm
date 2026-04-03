<script lang="ts">
    import '$lib/assets/css/variables.css';
    import '$lib/assets/css/standardControls.css';
    
	import type { PlatformAPI } from './index.ts';
	import { setAPI } from './core/context.ts';
	import Router from './Router.svelte';
	import { createApiClient } from './api/client.ts';

	let { api }: { api: PlatformAPI } = $props();

	$effect(() => {
		setAPI(api);
		createApiClient(api);
	});
</script>

<Router />
