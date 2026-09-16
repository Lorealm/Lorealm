<div class="realm-editor-shell">
	<div class="realm-editor-side">
		<!-- Asset Browser -->
	</div>
	<div class="realm-editor-pane-shell">
		<!-- Panes go here -->
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
	}
	.realm-editor-side {
		display: flex;
		height: 100%;
		width: 300px;
		background-color: var(--color-bg-elevated);
		border-right: 1px solid var(--color-border-subtle);
	}
	.realm-editor-pane-shell {
		display: flex;
		height: 100%;
		flex: 1;
	}
	.realm-editor-bottom {
		height: 200px;
		background-color: var(--color-bg-elevated);
		border-top: 1px solid var(--color-border-subtle);
	}
</style>
