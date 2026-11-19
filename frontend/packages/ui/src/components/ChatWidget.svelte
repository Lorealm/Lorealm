<script lang="ts">
	import type { WidgetInstance } from '#lib/types/widgets.js';
	import type { JsonValue } from '#lib/types/core.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import Unknown from '#lib/assets/widgets/Unknown.svelte';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import StateFrame from './StateFrame.svelte';

	let { widget, key }: { widget: WidgetInstance; key: string } = $props();

	const source = $derived(
		[...(userWidgetStore.loadedWidgets ?? []), ...(userWidgetStore.builtinWidgets ?? [])].find(
			(w) => w.id === widget.widget_id
		)
	);
</script>

{#if source}
	<StateFrame
		source={source.source}
		title={source.name}
		kind="props"
		slice={widget.props as Record<string, JsonValue>}
		onaction={(action, label) => chatStore.queueAction(key, label, action)}
	/>
{:else}
	<Unknown widgetId={widget.widget_id} />
{/if}
