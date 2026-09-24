<script lang="ts">
	import type { PluginSlot } from '$lib/stores/plugins.svelte.js';
	import type { JsonValue } from '$lib/types/core.js';
	import PluginFrame from './PluginFrame.svelte';
	import PluginMount from './PluginMount.svelte';

	let {
		slot,
		context,
		fill = false
	}: {
		slot: PluginSlot;
		context: JsonValue;
		/** Fill the parent, instead of growing to fit the content. */
		fill?: boolean;
	} = $props();
</script>

{#if slot.unsafe}
	<PluginMount handle={slot.handle} entry={slot.entry} {context} />
{:else}
	<PluginFrame
		handle={slot.handle}
		entry={slot.entry}
		title={slot.title}
		{context}
		{fill}
		autoplay={slot.autoplay}
	/>
{/if}
