<script lang="ts">
	import { TriangleAlert, Info, X } from '@lucide/svelte';
	import DOMPurify from 'dompurify';
	import { Marked } from 'marked';
	import { closePopup, popups, type PopupAction } from '$lib/stores/popup.svelte';

	// Single newlines break lines
	const markdown = new Marked({ breaks: true });

	const popup = $derived(popups[0]);
	const html = $derived(
		popup ? DOMPurify.sanitize(markdown.parse(popup.message, { async: false })) : ''
	);

	function openLinksOutside(event: MouseEvent) {
		const link = (event.target as Element).closest('a');
		if (!link?.href) return;
		event.preventDefault();
		window.open(link.href, '_blank', 'noopener');
	}
	let busy = $state(false);

	function dismiss() {
		if (popup?.dismissible && !busy) closePopup(popup.id, null);
	}

	async function choose(action: PopupAction) {
		if (!popup || busy) return;
		const id = popup.id;
		busy = true;
		try {
			await action.run?.();
		} finally {
			busy = false;
			closePopup(id, action.label);
		}
	}
</script>

<svelte:window onkeydown={(e) => popup && e.key === 'Escape' && dismiss()} />

{#if popup}
	{#key popup.id}
		<div
			class="overlay"
			role="presentation"
			onclick={(e) => e.currentTarget === e.target && dismiss()}
		>
			<div
				class="popup {popup.variant}"
				role="alertdialog"
				aria-modal="true"
				aria-labelledby="popup-title-{popup.id}"
				aria-describedby="popup-message-{popup.id}"
			>
				<header>
					{#if popup.variant === 'info'}
						<Info size={18} class="icon" />
					{:else}
						<TriangleAlert size={18} class="icon" />
					{/if}
					<h2 id="popup-title-{popup.id}">{popup.title}</h2>
					{#if popup.dismissible}
						<button type="button" class="close" aria-label="Close" onclick={dismiss}>
							<X size={14} />
						</button>
					{/if}
				</header>

				<div class="message" id="popup-message-{popup.id}" onclickcapture={openLinksOutside}>
					<!-- SAFETY: Sanitized with DOMPurify. -->
					<!-- eslint-disable-next-line svelte/no-at-html-tags -->
					{@html html}
				</div>

				<footer>
					{#each popup.actions as action, i (i)}
						<button
							type="button"
							class={action.tone ?? 'neutral'}
							disabled={busy}
							onclick={() => choose(action)}
						>
							{action.label}
						</button>
					{/each}
				</footer>
			</div>
		</div>
	{/key}
{/if}

<style>
	.overlay {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-md);
		background-color: var(--color-bg-overlay);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
	}
	.popup {
		display: flex;
		flex-direction: column;
		width: min(480px, 100%);
		max-height: 80vh;
		background-color: var(--color-bg-modal);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-xl);
	}
	.popup.warning {
		border-color: var(--color-warning-border);
	}
	.popup.danger {
		border-color: var(--color-error-border);
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.popup :global(.icon) {
		flex: 0 0 auto;
		color: var(--color-text-accent);
	}
	.popup.warning :global(.icon) {
		color: var(--color-warning-text);
	}
	.popup.danger :global(.icon) {
		color: var(--color-error-text);
	}
	h2 {
		flex: 1 1 auto;
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h5);
		font-weight: var(--font-weight-semibold);
	}
	.close {
		display: flex;
		padding: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-muted);
	}
	.close:hover {
		color: var(--color-text-accent);
	}
	.message {
		padding: var(--spacing-md);
		overflow-y: auto;
		color: var(--color-text-secondary);
	}
	.message :global(:is(h1, h2, h3, h4)) {
		margin: var(--spacing-sm) 0 var(--spacing-xs);
		font-size: var(--font-size-h5);
		color: var(--color-text-primary);
	}
	.message :global(:is(p, ul, ol, pre, blockquote, table)) {
		margin: 0 0 var(--spacing-sm);
	}
	.message :global(:is(ul, ol)) {
		padding-left: var(--spacing-lg);
	}
	.message :global(> :last-child) {
		margin-bottom: 0;
	}
	.message :global(strong) {
		color: var(--color-text-primary);
	}
	.message :global(a) {
		color: var(--color-text-accent);
	}
	.message :global(code) {
		font-family: var(--font-family-mono);
		font-size: 0.9em;
	}
	.message :global(pre) {
		overflow-x: auto;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-input);
	}
	.message :global(blockquote) {
		padding-left: var(--spacing-sm);
		border-left: 2px solid var(--color-border-medium);
	}
	.message :global(hr) {
		border: none;
		border-top: 1px solid var(--color-border-subtle);
		margin: var(--spacing-sm) 0;
	}
	footer {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-top: 1px solid var(--color-border-subtle);
	}
	button.neutral {
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
	button.danger {
		background: var(--color-error-bg);
		color: var(--color-error-text);
		border: 1px solid var(--color-error-border);
	}
	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
