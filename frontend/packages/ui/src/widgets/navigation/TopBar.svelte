<script lang="ts">
	import { ChevronLeft, ChevronRight } from '@lucide/svelte';
	import { crumbsFor } from '$lib/navigation/breadcrumbs.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { router } from '$lib/navigation/router.svelte.js';

	const crumbs = $derived(crumbsFor(router.current));
</script>

<div class="topBar">
	<div class="history">
		<button
			class="step"
			title="Back"
			aria-label="Back"
			disabled={!router.canGoBack}
			onclick={() => navigator.back()}
		>
			<ChevronLeft size={18} />
		</button>
		<button
			class="step"
			title="Forward"
			aria-label="Forward"
			disabled={!router.canGoForward}
			onclick={() => navigator.forward()}
		>
			<ChevronRight size={18} />
		</button>
	</div>

	<nav class="trail" aria-label="Breadcrumb">
		{#each crumbs as crumb, i (i)}
			{#if i > 0}
				<span class="separator" aria-hidden="true"><ChevronRight size={13} /></span>
			{/if}
			{#if crumb.go}
				<button class="crumb" onclick={crumb.go}>{crumb.label}</button>
			{:else}
				<span
					class="crumb"
					class:current={i === crumbs.length - 1}
					aria-current={i === crumbs.length - 1 ? 'page' : undefined}
				>
					{crumb.label}
				</span>
			{/if}
		{/each}
	</nav>
</div>

<style>
	.topBar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		width: 100%;
		flex-shrink: 0;
		height: var(--height-header);
		padding: 0 var(--spacing-sm);
		box-sizing: border-box;
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.history {
		display: flex;
		flex-shrink: 0;
		gap: 2px;
	}
	.step {
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xs);
		background: transparent;
		color: var(--color-text-secondary);
	}
	.step:not(:disabled) {
		cursor: pointer;
	}
	.step:not(:disabled):hover,
	.step:focus-visible {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.step:disabled {
		color: var(--color-text-disabled);
		background: transparent;
		cursor: default;
	}
	.trail {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		min-width: 0;
		overflow: hidden;
	}
	.crumb {
		flex-shrink: 0;
		padding: 2px var(--spacing-xs);
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		line-height: 1.6;
		white-space: nowrap;
	}
	/* Only the crumbs that lead somewhere take a button's shape, and only under
	   the cursor — a resting trail should read as text. */
	button.crumb {
		cursor: pointer;
	}
	button.crumb:hover,
	button.crumb:focus-visible {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	/* The tail is the one crumb that can lose its name to an ellipsis. */
	.crumb.current {
		flex-shrink: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		color: var(--color-text-primary);
	}
	.separator {
		display: flex;
		flex-shrink: 0;
		align-items: center;
		color: var(--color-text-disabled);
	}
</style>
