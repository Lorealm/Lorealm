<script lang="ts">
	import { useApi } from '#lib/api/client.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import type { JsonValue, Snowflake } from '#lib/types/core.js';
	import type { UserWidget } from '#lib/types/widgets.js';
	import StateFrame from './StateFrame.svelte';

	let { messageId, last = false }: { messageId: Snowflake; last?: boolean } = $props();

	const api = useApi();

	/** Whose eyes the widgets look through. Nobody's, in director view. */
	const viewer = $derived(chatStore.directorView ? null : chatStore.viewerAgent);

	const widgets = $derived.by((): UserWidget[] => {
		const all = userWidgetStore.loadedWidgets ?? [];
		if (!chatStore.conversationId) return [];
		// What you have switched on, plus what the cast brings with it.
		const bound = new Set([
			...(viewer ? [viewer] : (chatStore.cast?.agents ?? [])).flatMap((a) => a.widgets),
			...(chatStore.cast?.character_widgets ?? [])
		]);

		return all.filter(
			(w) =>
				(w.enabled || bound.has(w.id)) &&
				w.placement !== 'attachment' &&
				(w.mode === 'every' || last)
		);
	});

	let state = $state<Record<string, JsonValue>>({});

	$effect(() => {
		const convoId = chatStore.conversationId;
		const handle = viewer?.handle;
		if (!convoId || widgets.length === 0 || messageId.startsWith('tmp_')) return;

		void (async () => {
			try {
				state = (
					handle
						? await api.chat.agents.state(convoId, handle, messageId)
						: ((await api.chat.messages.state(convoId, messageId))?.persistent ?? {})
				) as Record<string, JsonValue>;
			} catch {
				// A missing state row just leaves the widgets on their last data.
			}
		})();
	});
</script>

{#each widgets as widget (widget.id)}
	<StateFrame
		source={widget.source}
		title={widget.name}
		slice={state}
		onaction={(action, label) => chatStore.queueAction(`static:${widget.id}`, label, action)}
	/>
{/each}
