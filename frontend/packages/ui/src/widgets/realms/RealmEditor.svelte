<script lang="ts">
	import AssetBrowser from './AssetBrowser.svelte';
	import PaneShell from './panes/PaneShell.svelte';
</script>

<div class="realm-editor-shell">
	<div class="realm-editor-main">
		<div class="realm-editor-side">
			<AssetBrowser />
		</div>
		<div class="realm-editor-pane-shell">
			<PaneShell />
		</div>
	</div>
	<div class="realm-editor-bottom">
		<!-- Diagnostics -->
	</div>
</div>

<style>
	.realm-editor-shell {
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		min-height: 0;
	}
	.realm-editor-main {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	.realm-editor-side {
		display: flex;
		flex: 0 0 auto;
		width: var(--width-sidebar-small);
		min-height: 0;
		background-color: var(--color-bg-elevated);
		border-right: 1px solid var(--color-border-subtle);
	}
	.realm-editor-pane-shell {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
	.realm-editor-bottom {
		flex: 0 0 auto;
		height: 200px;
		background-color: var(--color-bg-elevated);
		border-top: 1px solid var(--color-border-subtle);
	}
</style>
