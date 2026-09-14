<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		title,
		onclose,
		children,
		footer
	}: {
		title: string;
		onclose: () => void;
		children: Snippet;
		footer?: Snippet;
	} = $props();

	$effect(() => {
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') onclose();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="overlay"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="panel" role="dialog" aria-modal="true" aria-label={title}>
		<header class="panel-header">
			<h3>{title}</h3>
			<button class="close" type="button" aria-label="Close" onclick={onclose}>×</button>
		</header>

		<div class="panel-body">
			{@render children()}
		</div>

		{#if footer}
			<footer class="panel-footer">
				{@render footer()}
			</footer>
		{/if}
	</div>
</div>

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-lg);
		background-color: var(--color-overlay-bg);
		-webkit-backdrop-filter: var(--overlay-blur);
		backdrop-filter: var(--overlay-blur);
	}
	.panel {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: var(--width-content-narrow);
		max-height: 80vh;
		background-color: var(--color-bg-modal);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-xl);
	}
	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.panel-header h3 {
		margin: 0;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h4);
	}
	.close {
		flex: 0 0 auto;
		background-color: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-lg);
		line-height: 1;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.close:hover {
		color: var(--color-text-accent);
	}
	.panel-body {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		overflow-y: auto;
	}
	.panel-footer {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
	}
</style>
