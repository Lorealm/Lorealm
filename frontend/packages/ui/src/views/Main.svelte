<script>
	import Inspector from '$lib/widgets/inspector/Inspector.svelte';
	import NavBar from '$lib/widgets/navigation/NavBar.svelte';
	import MainViewRouter from '$lib/widgets/routers/MainViewRouter.svelte';

	import { provideApiClient } from '$lib/api/client.js';
	import { getPlatformApi } from '$lib/core/context.js';

	const platform = getPlatformApi();
	provideApiClient(platform);
</script>

<div class="container">
	<NavBar />
	<div id="mainView" class="centre">
		<div class="topBar"></div>
		<MainViewRouter />
	</div>
	<Inspector />
</div>

<style>
	.container {
		display: flex;
		flex-direction: row;
		width: 100vw;
		height: 100vh;
		min-height: 0;
		background-color: var(--color-bg-base);
	}
	#mainView {
		display: flex;
		flex-direction: column; /* topBar, StudioNav, editor stack vertically */
		min-height: 0; /* ← the key line: lets it shrink so children scroll */
		min-width: 0;
	}
	.topBar {
		display: flex;
		width: 100%;
		flex-shrink: 0;
		height: var(--height-header);
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.centre {
		display: flex;
		flex: 1;
		flex-direction: column;
		height: 100%;
	}
</style>
