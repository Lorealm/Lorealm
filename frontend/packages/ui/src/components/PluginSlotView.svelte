<script lang="ts">
	import type { PluginSlot } from '#lib/stores/plugins.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { ConversationMessage } from '#lib/types/conversation.js';
	import PluginFrame from './PluginFrame.svelte';
	import PluginMount from './PluginMount.svelte';

	let {
		slot,
		conversation = null,
		message = null,
		fill = false
	}: {
		slot: PluginSlot;
		/** The conversation the slot is showing, if the app is in one. */
		conversation?: Snowflake | null;
		/** The message the slot sits under, for a slot that sits under one. */
		message?: ConversationMessage | null;
		/** Fill the parent, instead of growing to fit the content. */
		fill?: boolean;
	} = $props();

	/** What the slot tells the plugin. Every slot says where it is. */
	const context = $derived({
		conversation,
		message: message
			? {
					id: message.id,
					role: message.role,
					content: message.content,
					updated_at: message.updated_at
				}
			: null
	});
</script>

{#if slot.unsafe}
	<PluginMount
		handle={slot.handle}
		entry={slot.entry}
		{context}
		{conversation}
		message={message?.id ?? null}
	/>
{:else}
	<PluginFrame
		handle={slot.handle}
		entry={slot.entry}
		title={slot.title}
		{context}
		{conversation}
		message={message?.id ?? null}
		{fill}
		autoplay={slot.autoplay}
	/>
{/if}
