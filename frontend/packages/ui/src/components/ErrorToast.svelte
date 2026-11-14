<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { dismissToast, toasts } from '#lib/stores/errorToast.svelte.js';
</script>

<div class="toast-container" aria-live="polite" aria-atomic="true">
	{#each toasts as toast (toast.id)}
		<article
			class="toast-card variant-{toast.variant}"
			role="status"
			style="--toast-duration: {toast.durationMs}ms;"
		>
			<div class="toast-content-wrapper">
				<div class="toast-text-body">
					<p class="toast-title">{toast.title}</p>
					<p class="toast-message">{toast.message}</p>
				</div>

				<button
					class="toast-dismiss-btn"
					onclick={() => dismissToast(toast.id)}
					aria-label={t.components.toast.dismiss}
				>
					✕
				</button>
			</div>

			{#if toast.actions.length > 0}
				<div class="toast-action-wrapper">
					{#each toast.actions as action (action.label)}
						{@const Icon = action.icon}
						<button
							class="toast-action-btn tone-{action.tone ?? 'neutral'}"
							onclick={() => {
								action.run();
								dismissToast(toast.id);
							}}
						>
							{#if Icon}
								<Icon size={14} aria-hidden="true" />
							{/if}
							<span>{action.label}</span>
						</button>
					{/each}
				</div>
			{/if}

			{#if toast.durationMs > 0}
				<div class="toast-progress-bar"></div>
			{/if}
		</article>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		bottom: var(--spacing-lg);
		right: var(--spacing-lg);
		z-index: var(--z-index-toast);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: min(24rem, calc(100vw - 2rem));
		font-family: var(--font-family-body);
	}

	.toast-card {
		position: relative;
		background-color: var(--color-bg-elevated);
		border-radius: var(--radius-xl);
		padding: var(--spacing-lg);
		box-shadow: var(--shadow-xl);
		overflow: hidden;
		border: 1px solid var(--variant-border);
	}

	.variant-error {
		--variant-text: var(--color-error-text);
		--variant-border: var(--color-error-border);
		--variant-progress: var(--color-error);
	}
	.variant-info {
		--variant-text: var(--color-info-text);
		--variant-border: var(--color-info-border);
		--variant-progress: var(--color-info);
	}
	.variant-warning {
		--variant-text: var(--color-warning-text);
		--variant-border: var(--color-warning-border);
		--variant-progress: var(--color-warning);
	}
	.variant-success {
		--variant-text: var(--color-success-text);
		--variant-border: var(--color-success-border);
		--variant-progress: var(--color-success);
	}

	.toast-content-wrapper {
		display: flex;
		align-items: start;
		justify-content: space-between;
		gap: var(--spacing-md);
		margin-bottom: var(--spacing-xs);
	}

	.toast-text-body {
		min-width: 0;
		flex: 1;
	}

	.toast-title {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
		font-weight: var(--font-weight-semibold);
		color: var(--variant-text);
		line-height: var(--line-height-tight);
	}

	.toast-message {
		margin: var(--spacing-xs) 0 0 0;
		font-size: var(--font-size-sm);
		color: var(--color-text-secondary);
		line-height: var(--line-height-snug);
	}

	.toast-dismiss-btn {
		background: none;
		border: none;
		cursor: pointer;
		padding: var(--spacing-xs) var(--spacing-sm);
		border-radius: var(--radius-md);
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
		transition:
			background-color var(--transition-fast),
			color var(--transition-fast);
	}

	.toast-dismiss-btn:hover {
		background-color: var(--color-hover);
		color: var(--color-text-primary);
	}

	.toast-action-wrapper {
		margin-top: var(--spacing-md);
		margin-bottom: var(--spacing-xs);
		display: flex;
		gap: var(--spacing-sm);
	}

	.toast-action-btn {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		background: none;
		color: var(--action-text);
		border: 1px solid var(--action-border);
		cursor: pointer;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-medium);
		padding: var(--spacing-sm) var(--spacing-md);
		border-radius: var(--radius-lg);
		transition: background-color var(--transition-fast);
	}

	.tone-neutral {
		--action-text: var(--variant-text);
		--action-border: var(--variant-border);
	}
	.tone-primary {
		--action-text: var(--color-primary);
		--action-border: var(--color-primary);
	}
	.tone-danger {
		--action-text: var(--color-error-text);
		--action-border: var(--color-error-border);
	}

	.toast-action-btn span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.toast-action-btn:hover {
		background-color: var(--color-hover);
	}

	/* Progress Bar Logic */
	.toast-progress-bar {
		position: absolute;
		bottom: 0;
		left: 0;
		height: 3px;
		width: 100%;
		background-color: var(--variant-progress);
		opacity: 0.75;
		transform-origin: left;
		animation: deplete var(--toast-duration) linear forwards;
	}

	@keyframes deplete {
		from {
			transform: scaleX(1);
		}
		to {
			transform: scaleX(0);
		}
	}
</style>
