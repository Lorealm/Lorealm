<script lang="ts">
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import type { ConversationMessage } from '$lib/types/conversation.js';
	import PluginSlotView from './PluginSlotView.svelte';

	let { message, last = false }: { message: ConversationMessage; last?: boolean } = $props();

	$effect(() => {
		void pluginsStore.load();
	});

	const slots = $derived(pluginsStore.slots('message').filter((s) => s.mode === 'every' || last));
	const context = $derived({
		message: {
			id: message.id,
			role: message.role,
			content: message.content,
			updated_at: message.updated_at
		}
	});
</script>

{#each slots as slot (slot.key)}
	<PluginSlotView {slot} {context} />
{/each}
