<script lang="ts">
	import type { WidgetInstance } from '$lib/types/widgets.js';
	import type { JsonValue } from '$lib/types/core.js';
	import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
	import Unknown from '$lib/assets/widgets/Unknown.svelte';
	import StateFrame from './StateFrame.svelte';

	let { widget }: { widget: WidgetInstance } = $props();

	const source = $derived(
		(userWidgetStore.loadedWidgets ?? []).find((w) => w.id === widget.widget_id)
	);
</script>

{#if source}
	<StateFrame
		source={source.source}
		title={source.name}
		kind="props"
		slice={widget.props as Record<string, JsonValue>}
	/>
{:else}
	<Unknown widgetId={widget.widget_id} />
{/if}
