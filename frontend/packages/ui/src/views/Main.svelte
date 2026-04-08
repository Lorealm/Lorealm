<script>
	import Inspector from '$lib/widgets/inspector/Inspector.svelte';
	import NavBar from '$lib/widgets/navigation/NavBar.svelte';
	import MainViewRouter from '$lib/widgets/routers/MainViewRouter.svelte';
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
		flex: 1;
		flex-direction: row;
		width: 100%;
		height: 100%;
		background-color: var(--color-bg-base);
	}
	.topBar {
		display: flex;
		width: 100%;
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
