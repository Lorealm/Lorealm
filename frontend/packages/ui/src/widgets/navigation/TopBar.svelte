<script lang="ts">
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { ChevronLeft, ChevronRight, MenuIcon, PanelRight } from '@lucide/svelte';
	import { crumbsFor } from '#lib/navigation/breadcrumbs.svelte.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import { router } from '#lib/navigation/router.svelte.js';
	import { shell } from '#lib/stores/shell.svelte.js';

	const crumbs = $derived(crumbsFor(router.current));
</script>

<div class="topBar">
	{#if shell.narrow}
		<button class="step" aria-label="Open menu" onclick={() => shell.openNav()}>
			<MenuIcon size={20} />
		</button>
	{/if}
	<div class="history">
		<button
			class="step"
			{@attach tooltip('Back')}
			aria-label="Back"
			disabled={!router.canGoBack}
			onclick={() => navigator.back()}
		>
			<ChevronLeft size={18} />
		</button>
		<button
			class="step forward"
			{@attach tooltip('Forward')}
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

	{#if shell.narrow}
		<button class="step panel" aria-label="Open inspector" onclick={() => shell.openInspector()}>
			<PanelRight size={20} />
		</button>
	{/if}
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
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
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
	.panel {
		margin-left: auto;
	}
	@media (max-width: 768px) {
		.topBar {
			gap: var(--spacing-xs);
			padding: 0 var(--spacing-xs);
		}
		.step {
			flex-shrink: 0;
			min-width: 40px;
			min-height: 40px;
		}
		/* The browser's own back and forward cover this, and the row is short. */
		.forward {
			display: none;
		}
		.trail {
			overflow-x: auto;
			scrollbar-width: none;
		}
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
