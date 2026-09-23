<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
	import type { JsonValue, Snowflake } from '$lib/types/core.js';
	import type { UserWidget } from '$lib/types/widgets.js';
	import StateFrame from './StateFrame.svelte';

	let { messageId, last = false }: { messageId: Snowflake; last?: boolean } = $props();

	const api = useApi();

	const widgets = $derived.by((): UserWidget[] => {
		const all = userWidgetStore.loadedWidgets ?? [];
		if (!chatStore.conversationId) return [];
		const bound = chatStore.isRealm
			? (() => {
					const viewer = chatStore.viewerAgent;
					const ids = new Set(
						viewer && !chatStore.realmDirectorView
							? viewer.widgets
							: (chatStore.realmCast?.agents ?? []).flatMap((a) => a.widgets)
					);
					return all.filter((w) => ids.has(w.id));
				})()
			: all.filter((w) => w.enabled);

		return bound.filter((w) => w.placement !== 'attachment' && (w.mode === 'every' || last));
	});

	let state = $state<Record<string, JsonValue>>({});

	$effect(() => {
		const convoId = chatStore.conversationId;
		const viewer = chatStore.realmDirectorView ? null : chatStore.viewerAgent;
		if (!convoId || widgets.length === 0 || messageId.startsWith('tmp_')) return;

		void (async () => {
			try {
				state = (
					chatStore.isRealm && viewer
						? await api.chat.realm.agents.state(convoId, viewer.handle, messageId)
						: ((await api.chat.messages.state(convoId, messageId))?.persistent ?? {})
				) as Record<string, JsonValue>;
			} catch {
				// A missing state row just leaves the widgets on their last data.
			}
		})();
	});

	function onAction(toolId: string, args: JsonValue) {
		void chatStore.submitRealmPlayerTurn(null, { tool_id: toolId, args });
	}
</script>

{#each widgets as widget (widget.id)}
	<StateFrame source={widget.source} title={widget.name} slice={state} onaction={onAction} />
{/each}
