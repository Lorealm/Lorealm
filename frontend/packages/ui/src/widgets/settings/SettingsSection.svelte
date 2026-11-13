<script lang="ts">
	import type { Snippet } from 'svelte';
	import { configStore, type SaveState } from '#lib/stores/serverConfig.svelte.js';
	import SaveIndicator from '#lib/components/SaveIndicator.svelte';

	let {
		title,
		dirty,
		saveState,
		onsave,
		onreset,
		children
	}: {
		title: string;
		dirty: boolean;
		saveState: SaveState;
		onsave: () => void;
		onreset: () => void;
		children: Snippet;
	} = $props();
</script>

<section class="section">
	<fieldset class="body" disabled={!configStore.canEditServer}>
		<h2>{title}</h2>
		{@render children()}
	</fieldset>

	<footer>
		<SaveIndicator state={saveState} />
		<div class="spacer"></div>
		{#if configStore.canEditServer}
			<button type="button" class="ghost" onclick={onreset} disabled={!dirty}>Reset</button>
			<button
				type="button"
				class="primary"
				onclick={onsave}
				disabled={!dirty || saveState.status === 'saving'}
			>
				Save
			</button>
		{/if}
	</footer>
</section>

<style>
	.section {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
		flex-direction: column;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		min-inline-size: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-lg);
		margin: 0;
		padding: var(--spacing-lg);
		border: none;
	}
	.body h2 {
		font-size: var(--font-size-h4);
	}
	footer {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex-shrink: 0;
		padding: var(--spacing-md) var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.spacer {
		flex: 1 1 auto;
	}
	button.ghost {
		background: none;
	}
	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
