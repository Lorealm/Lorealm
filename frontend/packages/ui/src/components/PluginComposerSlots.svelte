<script lang="ts">
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import PluginSlotView from './PluginSlotView.svelte';

	$effect(() => {
		void pluginsStore.load();
	});

	const slots = $derived(pluginsStore.slots('composer'));
</script>

{#if slots.length > 0}
	<div class="slots">
		{#each slots as slot (slot.key)}
			<PluginSlotView {slot} conversation={chatStore.conversationId} />
		{/each}
	</div>
{/if}

<style>
	.slots {
		flex-shrink: 0;
		padding: 0 var(--spacing-md);
		background-color: var(--color-bg-base);
	}
</style>
