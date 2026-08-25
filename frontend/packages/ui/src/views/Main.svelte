<script lang="ts">
	import Inspector from '$lib/widgets/inspector/Inspector.svelte';
	import NavBar from '$lib/widgets/navigation/NavBar.svelte';
	import TopBar from '$lib/widgets/navigation/TopBar.svelte';
	import MainViewRouter from '$lib/widgets/routers/MainViewRouter.svelte';

	import ErrorToast from '$lib/components/ErrorToast.svelte';
	import { builtInCommands } from '$lib/keys/commands.js';
	import { keybinds } from '$lib/keys/manager.svelte.js';
	import { commandRegistry } from '$lib/keys/registry.svelte.js';

	$effect(() => {
		const unregister = commandRegistry.register(builtInCommands);
		const uninstall = keybinds.install();
		return () => {
			uninstall();
			unregister();
		};
	});
</script>

<div class="container">
	<NavBar />
	<div id="mainView" class="centre">
		<TopBar />
		<MainViewRouter />
	</div>
	<Inspector />
</div>
<ErrorToast />

<style>
	.container {
		display: flex;
		flex-direction: row;
		width: 100%;
		height: 100%;
		min-height: 0;
		background-color: var(--color-bg-base);
	}
	#mainView {
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
	}
	.centre {
		display: flex;
		flex: 1;
		flex-direction: column;
		height: 100%;
	}
</style>
