<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import type { PlatformAPI } from '@lorealm/ui';

	let { api }: { api: PlatformAPI } = $props();

	// svelte-ignore state_referenced_locally
	const overlay = api.capabilities.has('titlebar-overlay');
	// svelte-ignore state_referenced_locally
	const controls = api.capabilities.has('window-controls');
	const show = overlay || controls;
</script>

{#if show}
	<div class="titlebar" class:overlay>
		<!-- The only drag region: a flex spacer. Keeps clicks on the buttons clean. -->
		<div class="drag" data-tauri-drag-region></div>

		{#if controls && api.window}
			<div class="controls">
				<button
					class="tb-btn"
					aria-label={t.shell.window.minimize}
					onclick={() => api.window!.minimize()}
				>
					<svg viewBox="0 0 12 12"><line x1="2" y1="6" x2="10" y2="6" /></svg>
				</button>
				<button
					class="tb-btn"
					aria-label={t.shell.window.maximize}
					onclick={() => api.window!.toggleMaximize()}
				>
					<svg viewBox="0 0 12 12"><rect x="2.5" y="2.5" width="7" height="7" /></svg>
				</button>
				<button
					class="tb-btn close"
					aria-label={t.shell.window.close}
					onclick={() => api.window!.close()}
				>
					<svg viewBox="0 0 12 12">
						<line x1="2.5" y1="2.5" x2="9.5" y2="9.5" />
						<line x1="9.5" y1="2.5" x2="2.5" y2="9.5" />
					</svg>
				</button>
			</div>
		{/if}
	</div>
{/if}

<style>
	.titlebar {
		flex: 0 0 auto;
		display: flex;
		align-items: stretch;
		justify-content: flex-end;
		height: var(--height-titlebar, 36px);
		background:
			linear-gradient(var(--color-bg-elevated), var(--color-bg-elevated)), var(--color-bg-base);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
		border-bottom: 1px solid var(--color-border-subtle);
		user-select: none;
		-webkit-user-select: none;
	}

	.titlebar.overlay {
		padding-left: 80px;
	}

	.drag {
		flex: 1 1 auto;
		height: 100%;
	}

	.controls {
		display: flex;
		align-items: stretch;
	}

	/* Reset the global button look — caption buttons are full-height, square, flat. */
	.titlebar .tb-btn {
		width: 46px;
		height: 100%;
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		border-radius: 0;
		color: var(--color-text-secondary);
		transition:
			background-color var(--transition-fast) var(--easing-default),
			color var(--transition-fast) var(--easing-default);
	}

	.titlebar .tb-btn:hover {
		background: var(--color-hover);
		color: var(--color-text-primary);
	}

	.titlebar .tb-btn.close:hover {
		background: var(--color-error);
		color: var(--color-text-white);
	}

	.tb-btn svg {
		width: 12px;
		height: 12px;
		fill: none;
		stroke: currentColor;
		stroke-width: 1.2;
		stroke-linecap: round;
	}
</style>
