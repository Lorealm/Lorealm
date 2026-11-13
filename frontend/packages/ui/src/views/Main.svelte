<script lang="ts">
	import Inspector from '#lib/widgets/inspector/Inspector.svelte';
	import NavBar from '#lib/widgets/navigation/NavBar.svelte';
	import TopBar from '#lib/widgets/navigation/TopBar.svelte';
	import MainViewRouter from '#lib/widgets/routers/MainViewRouter.svelte';

	import ErrorToast from '#lib/components/ErrorToast.svelte';
	import ConversationManager from '#lib/widgets/chat/ConversationManager.svelte';
	import { builtInSlashCommands, serverCommands, slashCommands } from '#lib/commands/index.js';
	import { builtInCommands } from '#lib/keys/commands.js';
	import { keybinds } from '#lib/keys/manager.svelte.js';
	import { commandRegistry } from '#lib/keys/registry.svelte.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import { shell } from '#lib/stores/shell.svelte.js';

	$effect(() => {
		const unregister = commandRegistry.register(builtInCommands);
		const unregisterSlash = slashCommands.register(builtInSlashCommands);
		const uninstall = keybinds.install();
		// Registered after the built-ins, and so able to shadow them.
		void serverCommands.load();
		return () => {
			uninstall();
			serverCommands.clear();
			unregisterSlash();
			unregister();
		};
	});

	$effect(() => {
		void router.current;
		shell.closeDrawer();
	});

	function closeOnEscape(event: KeyboardEvent) {
		if (event.key === 'Escape' && shell.drawerOpen) shell.closeDrawer();
	}
</script>

<svelte:window onkeydown={closeOnEscape} />

<div class="container">
	{#if shell.drawerOpen}
		<button type="button" class="scrim" aria-label="Close panel" onclick={() => shell.closeDrawer()}
		></button>
	{/if}
	<NavBar />
	<div id="mainView" class="centre">
		<TopBar />
		<MainViewRouter />
	</div>
	<Inspector />
</div>
<ErrorToast />
<ConversationManager />

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
	.scrim {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-sticky);
		padding: 0;
		border: none;
		border-radius: 0;
		background-color: var(--color-overlay-bg);
	}
	.scrim:hover {
		background-color: var(--color-overlay-bg);
	}
</style>
