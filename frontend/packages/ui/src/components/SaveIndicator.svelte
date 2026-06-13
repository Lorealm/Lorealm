<script lang="ts">
	import type { SaveState } from '$lib/stores/lorebook.svelte.js';

	let { state }: { state: SaveState } = $props();
</script>

<div
	class="save-indicator"
	class:saving={state.status === 'saving'}
	class:saved={state.status === 'saved'}
	class:error={state.status === 'error'}
>
	{#if state.status === 'saving'}
		<span>Saving…</span>
	{:else if state.status === 'saved'}
		<span>Saved</span>
	{:else if state.status === 'error'}
		<span>Unsaved — invalid</span>
		{#if state.message}
			<details class="diagnostic">
				<summary>{state.code ?? 'Error'}</summary>
				<pre>{state.message}</pre>
			</details>
		{/if}
	{/if}
</div>

<style>
	.save-indicator {
		min-height: 1em;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.save-indicator.saving {
		color: var(--color-warning-text);
	}
	.save-indicator.saved {
		color: var(--color-success-text);
	}
	.save-indicator.error {
		color: var(--color-error-text);
	}
	.diagnostic {
		margin-top: var(--spacing-xs);
	}
	.diagnostic summary {
		cursor: pointer;
	}
	.diagnostic pre {
		margin: var(--spacing-xs) 0 0;
		padding: var(--spacing-sm);
		overflow-x: auto;
		font-family: var(--font-family-mono, ui-monospace, monospace);
		font-size: var(--font-size-sm);
		white-space: pre;
		border: 1px solid var(--color-border-subtle);
	}
</style>
