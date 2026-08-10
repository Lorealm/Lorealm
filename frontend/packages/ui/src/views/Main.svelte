<script>
	import Inspector from '$lib/widgets/inspector/Inspector.svelte';
	import NavBar from '$lib/widgets/navigation/NavBar.svelte';
	import MainViewRouter from '$lib/widgets/routers/MainViewRouter.svelte';

	import ErrorToast from '$lib/components/ErrorToast.svelte';
</script>

<div class="container">
	<NavBar />
	<div id="mainView" class="centre">
		<div class="topBar"></div>
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
