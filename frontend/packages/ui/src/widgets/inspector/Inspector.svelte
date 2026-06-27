<script>
	import CharacterImageWidget from './widgets/CharacterImageWidget.svelte';
	import ContextWidget from './widgets/ContextWidget.svelte';
	import StateWidget from './widgets/StateWidget.svelte';
</script>

<div class="container">
	<div class="topBar"></div>
	<div class="content">
		<CharacterImageWidget />
		<ContextWidget />
		<StateWidget />
	</div>
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
		flex-shrink: 0;
		width: 100%;
		height: var(--height-header);
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.content {
		display: flex;
		flex-direction: column;
		overflow-y: auto;
	}
</style>
