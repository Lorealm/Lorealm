<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import type { Snippet } from 'svelte';
	import { Minimize2 } from '@lucide/svelte';

	let {
		title,
		onclose,
		actions,
		children
	}: {
		title: string;
		onclose: () => void;
		/** Extra controls for the title bar, left of the close button. */
		actions?: Snippet;
		children: Snippet;
	} = $props();

	function onkeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation();
			onclose();
		}
	}
</script>

<svelte:window {onkeydown} />

<div class="overlay" role="dialog" aria-modal="true" aria-label={title}>
	<div class="bar">
		<span class="title">{title}</span>
		<div class="actions">
			{@render actions?.()}
			<button
				type="button"
				class="close"
				onclick={onclose}
				{@attach tooltip(t.components.maximize.minimizeHint)}
			>
				<Minimize2 size={14} />
				{t.components.maximize.minimize}
			</button>
		</div>
	</div>
	<div class="body">
		{@render children()}
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-xl);
		background-color: var(--color-bg-base);
	}
	.bar {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
	}
	.title {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.actions {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.close {
		align-items: center;
		gap: var(--spacing-xs);
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}
</style>
