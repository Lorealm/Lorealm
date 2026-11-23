<script lang="ts">
	import type { Snippet } from 'svelte';
	import { t } from '#lib/stores/i18n.svelte.js';

	let {
		prompt,
		action = t.common.delete,
		onConfirm,
		trigger
	}: {
		/** The question shown once armed, e.g. "Delete “Foo”?". */
		prompt: string;
		/** Label on the confirming button. */
		action?: string;
		onConfirm: () => unknown;
		/** The idle control. Call `arm` from its click handler to ask for confirmation. */
		trigger: Snippet<[arm: (event?: Event) => void]>;
	} = $props();

	let armed = $state(false);
	let cancelEl = $state<HTMLButtonElement>();

	function arm(event?: Event) {
		event?.stopPropagation();
		armed = true;
	}

	function cancel(event?: Event) {
		event?.stopPropagation();
		armed = false;
	}

	function confirm(event: Event) {
		event.stopPropagation();
		armed = false;
		void onConfirm();
	}

	$effect(() => {
		if (armed) cancelEl?.focus();
	});
</script>

{#if armed}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<span
		class="confirm-delete"
		role="group"
		aria-label={prompt}
		onclick={(e) => e.stopPropagation()}
		onkeydown={(e) => {
			e.stopPropagation();
			if (e.key === 'Escape') cancel();
		}}
	>
		<span class="prompt">{prompt}</span>
		<button type="button" class="yes" onclick={confirm}>{action}</button>
		<button type="button" class="no" bind:this={cancelEl} onclick={cancel}>
			{t.common.cancel}
		</button>
	</span>
{:else}
	{@render trigger(arm)}
{/if}

<style>
	.confirm-delete {
		display: inline-flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.prompt {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		overflow-wrap: anywhere;
	}
	button {
		padding: 2px var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		background: none;
		color: var(--color-text-primary);
		font-size: var(--font-size-sm);
		cursor: pointer;
		white-space: nowrap;
	}
	button:hover {
		border-color: var(--color-border-medium);
	}
	.yes {
		border-color: var(--color-error-text);
		background-color: var(--color-error-bg);
		color: var(--color-error-text);
	}
	.yes:hover {
		background-color: var(--color-error);
		color: var(--color-text-white);
	}
</style>
