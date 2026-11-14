<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import type { ContextNode } from '#lib/types/node.js';
	import Modal from './Modal.svelte';
	import NodeSettings from '../nodes/NodeSettings.svelte';

	let { node, onclose }: { node: ContextNode; onclose: () => void } = $props();

	let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
</script>

<Modal title={t.studio.templates.editNode} {onclose}>
	<NodeSettings {node} bind:saveState />

	{#snippet footer()}
		<span
			class="indicator"
			class:saving={saveState === 'saving'}
			class:saved={saveState === 'saved'}
			class:error={saveState === 'error'}
		>
			{#if saveState === 'saving'}{t.common.saving}{:else if saveState === 'saved'}{t.common
					.saved}{:else if saveState === 'error'}{t.studio.templates.saveFailed}{/if}
		</span>
		<button class="primary" type="button" onclick={onclose}>{t.studio.templates.done}</button>
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
