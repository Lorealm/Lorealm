<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../stores/tooltip.svelte.ts';
	import type { Snippet } from 'svelte';
	import { Maximize2 } from '@lucide/svelte';
	import MaximizeOverlay from './MaximizeOverlay.svelte';
	import TokenBadge from './TokenBadge.svelte';

	let {
		label,
		hint,
		tokenText,
		maximizable = true,
		fill = false,
		actions,
		editor
	}: {
		label?: string;
		/** Explanatory text behind a `?` next to the label. */
		hint?: string;
		/** Text the token badge measures. Omit on fields that never reach the model. */
		tokenText?: string;
		maximizable?: boolean;
		/** Stretch to the parent's height rather than sizing to the editor. */
		fill?: boolean;
		/** Extra controls for the header row, left of the token badge. */
		actions?: Snippet;
		/** The editor itself. `fill` is true when it should stretch to fill the overlay. */
		editor: Snippet<[boolean]>;
	} = $props();

	let maximized = $state(false);
</script>

<div class="field" class:fill>
	<div class="head">
		{#if label}<span class="label">{label}</span>{/if}
		{#if hint}
			<button
				type="button"
				class="hint"
				{@attach tooltip(hint)}
				aria-label={t.components.field.help(label ?? t.components.field.fallbackLabel)}>?</button
			>
		{/if}
		<span class="spacer"></span>
		{@render actions?.()}
		{#if tokenText !== undefined}<TokenBadge text={tokenText} />{/if}
		{#if maximizable}
			<button
				type="button"
				class="maximize"
				{@attach tooltip(t.components.field.maximize)}
				aria-label={t.components.field.maximizeField(label ?? t.components.field.fallbackLabel)}
				onclick={() => (maximized = true)}
			>
				<Maximize2 size={13} />
			</button>
		{/if}
	</div>

	{#if !maximized}
		{@render editor(false)}
	{:else}
		<div class="placeholder" aria-hidden="true">{t.components.field.editingFullScreen}</div>
	{/if}
</div>

{#if maximized}
	<MaximizeOverlay title={label ?? t.components.field.edit} onclose={() => (maximized = false)}>
		{#snippet actions()}
			{#if tokenText !== undefined}<TokenBadge text={tokenText} />{/if}
		{/snippet}
		<div class="overlayBody">
			{@render editor(true)}
		</div>
	</MaximizeOverlay>
{/if}

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.field.fill {
		flex: 1 1 auto;
		min-height: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		min-height: 20px;
	}
	.label {
		color: var(--color-text-secondary);
		font-family: var(--font-family-body);
		font-size: var(--font-size-base);
	}
	.spacer {
		flex: 1 1 auto;
	}
	.hint {
		flex: 0 0 auto;
		width: 16px;
		height: 16px;
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		line-height: 1;
		cursor: help;
	}
	.hint:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.maximize {
		flex: 0 0 auto;
		display: inline-flex;
		align-items: center;
		padding: 2px 5px;
		background-color: transparent;
		color: var(--color-text-muted);
		border-radius: var(--radius-sm);
	}
	.maximize:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.placeholder {
		display: flex;
		align-items: center;
		justify-content: center;
		border: 1px dashed var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-lg);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.overlayBody {
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
		flex-direction: column;
		width: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
	}
</style>
