<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { CONVERSATION_MANAGER_LAYER } from '$lib/keys/commands.js';
	import { keyLayer } from '$lib/keys/layers.svelte.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { conversationManager } from '$lib/stores/conversationManager.svelte.js';
	import { showApiError } from '$lib/stores/errorActions.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { ConversationMetadata } from '$lib/types/conversation.js';
	import { Trash2, X } from '@lucide/svelte';
	import { dateTimeMedium, relativeTime } from '$lib/assets/scripts/datetime.js';

	const api = useApi();

	let conversations = $state<ConversationMetadata[]>([]);
	let loading = $state(false);

	async function load(characterId: Snowflake) {
		loading = true;
		try {
			const chats = await api.characters.chats(characterId);
			if (conversationManager.characterId !== characterId) return; // stale
			conversations = [...chats].sort((a, b) => b.updated_at.localeCompare(a.updated_at));
		} catch (e) {
			showApiError(e, { title: 'Failed to load conversations' });
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const characterId = conversationManager.characterId;
		if (characterId) void load(characterId);
		else conversations = [];
	});

	/** Conversations carry no title of their own yet, so fall back to when it began. */
	function label(meta: ConversationMetadata): string {
		return meta.title ?? dateTimeMedium(meta.created_at) ?? meta.created_at;
	}

	async function open(meta: ConversationMetadata) {
		conversationManager.close();
		navigator.start().write();
		try {
			await chatStore.loadConversation(meta.id);
		} catch (e) {
			showApiError(e, { title: 'Failed to open conversation' });
		}
	}

	async function remove(meta: ConversationMetadata) {
		if (!confirm(`Delete "${label(meta)}"? This can't be undone.`)) return;
		try {
			await chatStore.deleteConversation(meta.id);
		} catch {
			return; // the store already reported it
		}
		conversations = conversations.filter((c) => c.id !== meta.id);
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Escape') conversationManager.close();
	}
</script>

<svelte:window onkeydown={onKey} />

{#if conversationManager.isOpen}
	<div
		use:keyLayer={{ id: CONVERSATION_MANAGER_LAYER, blocking: true }}
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) conversationManager.close();
		}}
	>
		<div class="modal" role="dialog" aria-modal="true" aria-label="Conversations">
			<div class="header">
				<h2>Conversations</h2>
				<button class="icon-btn" title="Close" onclick={() => conversationManager.close()}>
					<X />
				</button>
			</div>

			<div class="list">
				{#each conversations as meta (meta.id)}
					<div class="row" class:active={meta.id === chatStore.conversationId}>
						<button type="button" class="pick" onclick={() => open(meta)}>
							<span class="name">{label(meta)}</span>
							<span class="meta">
								{meta.length} message{meta.length === 1 ? '' : 's'}
								<span class="dot">·</span>
								{relativeTime(meta.updated_at) ?? ''}
							</span>
						</button>
						<button
							type="button"
							class="icon-btn"
							title="Delete conversation"
							onclick={() => remove(meta)}
						>
							<Trash2 size={16} />
						</button>
					</div>
				{:else}
					<div class="empty">
						{loading ? 'Loading…' : 'No conversations with this character yet.'}
					</div>
				{/each}
			</div>
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		z-index: var(--z-index-modal);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xl);
		background: var(--color-overlay-bg);
		-webkit-backdrop-filter: var(--overlay-blur);
		backdrop-filter: var(--overlay-blur);
	}

	.modal {
		display: flex;
		flex-direction: column;
		width: min(520px, 100%);
		max-height: 70vh;
		background: var(--color-bg-modal);
		border: 1px solid var(--color-border-medium);
		border-radius: var(--radius-2xl);
		box-shadow: var(--shadow-xl);
		overflow: hidden;
	}

	.header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.header h2 {
		margin: 0;
	}

	.list {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
		overflow-y: auto;
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-md);
	}

	.row:hover {
		background: var(--color-hover);
	}

	.row.active {
		background: var(--color-selected);
	}

	.pick {
		display: flex;
		flex: 1;
		flex-direction: column;
		align-items: flex-start;
		gap: 2px;
		min-width: 0;
		background: transparent;
		text-align: left;
	}

	.name {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
	}

	.meta {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	.dot {
		margin: 0 0.35em;
	}

	.icon-btn {
		flex: 0 0 auto;
		justify-content: center;
		align-items: center;
		background-color: transparent;
	}

	.empty {
		padding: var(--spacing-lg);
		text-align: center;
		color: var(--color-text-muted);
	}
</style>
