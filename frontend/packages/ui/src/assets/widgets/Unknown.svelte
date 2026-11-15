<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	let { widgetId }: { widgetId: string } = $props();
</script>

<div class="widget unknown">{t.components.unknownWidget({ id: widgetId })}</div>

<style>
	.widget {
		max-width: 75%;
	}

	.unknown {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		border: 1px dashed var(--color-border-subtle);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm);
		width: fit-content;
	}
</style>
