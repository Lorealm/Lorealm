<script lang="ts">
	import SettingsRouter from '../routers/SettingsRouter.svelte';

	let view = $state('none');

	function setView(newView: string) {
		view = newView;
	}
</script>

<div class="container">
	<div class="settings-nav">
		<button onclick={() => setView('roles')}>Roles</button>
	</div>
	<SettingsRouter {view} />
</div>

<style>
	.container {
		display: flex;
		flex: 1;
		flex-direction: row;
	}
	.settings-nav {
		display: flex;
		flex-direction: column;
		width: 10em;
		height: 100%;
		background-color: var(--color-bg-elevated);
		border-right: 1px solid var(--color-border-subtle);
	}
</style>
