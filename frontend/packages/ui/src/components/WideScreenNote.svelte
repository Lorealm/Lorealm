<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { shell } from '#lib/stores/shell.svelte.js';
	import { MonitorSmartphone, X } from '@lucide/svelte';

	let { subject }: { subject: string } = $props();

	let dismissed = $state(false);
</script>

{#if shell.narrow && !dismissed}
	<div class="note" role="note">
		<MonitorSmartphone size={16} aria-hidden="true" />
		<span>{t.components.wideScreenNote(subject)}</span>
		<button type="button" aria-label={t.common.dismiss} onclick={() => (dismissed = true)}>
			<X size={16} />
		</button>
	</div>
{/if}

<style>
	.note {
		display: flex;
		flex-shrink: 0;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-xs) var(--spacing-xs) var(--spacing-xs) var(--spacing-md);
		border-bottom: 1px solid var(--color-info-border);
		background-color: var(--color-info-bg);
		color: var(--color-info-text);
		font-size: var(--font-size-sm);
	}
	.note span {
		flex: 1 1 auto;
		min-width: 0;
	}
	.note button {
		flex-shrink: 0;
		align-items: center;
		justify-content: center;
		min-width: 36px;
		min-height: 36px;
		padding: 0;
		background: transparent;
		color: inherit;
	}
</style>
