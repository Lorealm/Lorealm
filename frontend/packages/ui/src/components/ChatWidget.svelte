<script lang="ts">
	import type { WidgetInstance } from '$lib/types/widgets.js';
	import { widgetRegistry } from '$lib/assets/widgets/registry.js';
	import Unknown from '$lib/assets/widgets/Unknown.svelte';

	let { widget }: { widget: WidgetInstance } = $props();

	const Component = $derived(widgetRegistry[widget.widget_id]);
</script>

{#if Component}
	<Component props={widget.props} />
{:else}
	<Unknown widgetId={widget.widget_id} />
{/if}
