<script>
	import ContextWidget from "./widgets/ContextWidget.svelte";

</script>
<div class="container">
	<div class="topBar"></div>
	<ContextWidget />
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		height: 100%;
		width: var(--width-sidebar-small);
		background-color: var(--color-bg-elevated);
		border-left: 1px solid var(--color-border-subtle);
	}
	.topBar {
		display: flex;
		width: 100%;
		height: var(--height-header);
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
	}
</style>
