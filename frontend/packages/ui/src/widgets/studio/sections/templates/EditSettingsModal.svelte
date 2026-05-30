<script lang="ts">
	import type { ContextNode } from '$lib/types/node.js';
	import Modal from './Modal.svelte';
	import NodeSettings from './NodeSettings.svelte';

	let { node, onclose }: { node: ContextNode; onclose: () => void } = $props();

	let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
</script>

<Modal title="Edit node" {onclose}>
	<NodeSettings {node} bind:saveState />

	{#snippet footer()}
		<span
			class="indicator"
			class:saving={saveState === 'saving'}
			class:saved={saveState === 'saved'}
			class:error={saveState === 'error'}
		>
			{#if saveState === 'saving'}Saving…{:else if saveState === 'saved'}Saved{:else if saveState === 'error'}Save
				failed{/if}
		</span>
		<button class="primary" type="button" onclick={onclose}>Done</button>
	{/snippet}
</Modal>

<style>
	.indicator {
		margin-right: auto;
		min-height: 1em;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.indicator.saving {
		color: var(--color-warning-text);
	}
	.indicator.saved {
		color: var(--color-success-text);
	}
	.indicator.error {
		color: var(--color-error-text);
	}
	.primary {
		background-color: var(--color-primary-dark);
		color: var(--color-text-white);
	}
	.primary:hover {
		background-color: var(--color-primary);
	}
</style>
