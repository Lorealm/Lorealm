<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import type { ConversationMessage } from '$lib/types/conversation.js';
	import TransportImage from '$lib/components/TransportImage.svelte';
	import { ChevronLeft, ChevronRight, Layers, Pencil, Trash2 } from '@lucide/svelte';
	import { contextInspector } from '$lib/stores/contextInspector.svelte.js';
	import { parseContextBlocks } from '$lib/types/context.js';

	let { message, last = false }: { message: ConversationMessage; last?: boolean } = $props();

	// Slide position drives the chevrons; recomputes as siblings appear.
	let info = $derived(chatStore.siblingInfo(message.id));
	let name = $derived(
		message.role === 'user'
			? 'You'
			: message.role === 'system'
				? 'System'
				: (characterStore.activeCharacter?.definition.name ?? 'Assistant')
	);
	// Provisional bubbles (greeting picker, in-flight stream) have temp ids and
	// can't be edited/deleted yet, so hide those actions for them.
	let committed = $derived(!message.id.startsWith('tmp_'));
	// The streaming bubble can be momentarily empty before the first token.
	let typing = $derived(last && chatStore.isStreaming && message.content.length === 0);
	let busy = $derived(chatStore.isStreaming);

	// One inline editor serves both in-place edits (`edit`) and user-branch
	// swipes (`branch`); only the commit differs.
	let editing = $state(false);
	let editMode = $state<'edit' | 'branch'>('edit');
	let editText = $state('');

	let hasContext = $derived(parseContextBlocks(message.metadata) !== null);

	function startEdit(mode: 'edit' | 'branch') {
		editMode = mode;
		editText = message.content;
		editing = true;
	}

	async function saveEdit() {
		const text = editText;
		editing = false;
		if (editMode === 'edit') await chatStore.editMessage(message.id, text);
		else await chatStore.commitUserSwipe(message.id, text);
	}

	// Chevron-right: step to the next slide, or — at the last slide — make a new
	// one. Assistant messages generate a fresh reply; user messages open the
	// editor to commit a new branch (Some/Some needs text).
	function onRight() {
		if (info.index < info.count - 1) {
			chatStore.swipeNext(message.id);
		} else if (message.role === 'assistant') {
			chatStore.swipeNext(message.id);
		} else {
			startEdit('branch');
		}
	}
</script>

<div class="message" data-role={message.role}>
	<div class="avatar">
		{#if message.role === 'assistant' && chatStore.characterId}
			<TransportImage src={`/api/v1/characters/${chatStore.characterId}/image`} alt={name} />
		{:else}
			<div class="avatar-placeholder"></div>
		{/if}
	</div>

	<div class="box">
		<div class="box-header">
			<span class="name">{name}</span>
			{#if committed}
				<div class="actions">
					{#if hasContext}
						<button
							class="icon-btn"
							title="Inspect context"
							onclick={() => contextInspector.open(message)}
						>
							<Layers />
						</button>
					{/if}
					<button class="icon-btn" title="Edit" disabled={busy} onclick={() => startEdit('edit')}>
						<Pencil />
					</button>
					<button
						class="icon-btn"
						title="Delete"
						disabled={busy}
						onclick={() => chatStore.deleteMessage(message.id)}
					>
						<Trash2 />
					</button>
				</div>
			{/if}
		</div>

		{#if editing}
			<textarea class="editor" rows="3" bind:value={editText}></textarea>
			<div class="edit-actions">
				<button onclick={saveEdit}>{editMode === 'edit' ? 'Save' : 'Swipe'}</button>
				<button onclick={() => (editing = false)}>Cancel</button>
			</div>
		{:else if typing}
			<div class="body">
				<span class="typing"><span></span><span></span><span></span></span>
			</div>
		{:else}
			<div class="body">{message.content}</div>
		{/if}

		<div class="slides">
			<button
				class="icon-btn"
				title="Previous"
				disabled={busy || info.index === 0}
				onclick={() => chatStore.swipePrev(message.id)}
			>
				<ChevronLeft />
			</button>
			{#if info.count > 1}
				<span class="counter">{info.index + 1}/{info.count}</span>
			{/if}
			<button class="icon-btn" title="Next" disabled={busy} onclick={onRight}>
				<ChevronRight />
			</button>
		</div>
	</div>
</div>

<style>
	.message {
		display: flex;
		flex-direction: row;
		align-items: flex-start;
		gap: var(--spacing-md);
	}

	.avatar {
		flex: 0 0 auto;
		width: 40px;
		height: 40px;
		border-radius: var(--radius-full);
		overflow: hidden;
		border: 2px solid var(--color-border-medium);
	}
	/* TransportImage's root <span> carries no intrinsic size; fill the avatar. */
	.avatar :global(.transport-image) {
		display: block;
		width: 100%;
		height: 100%;
	}
	.avatar-placeholder {
		width: 100%;
		height: 100%;
		background: var(--gradient-portrait);
	}
	.message[data-role='assistant'] .avatar {
		border-color: var(--color-role-assistant);
	}
	.message[data-role='user'] .avatar {
		border-color: var(--color-role-user);
	}
	.message[data-role='system'] .avatar {
		border-color: var(--color-role-system);
	}

	.box {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		gap: var(--spacing-sm);
		background-color: var(--color-bg-elevated);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-lg);
		padding: var(--spacing-md);
	}

	.box-header {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: space-between;
		gap: var(--spacing-sm);
	}
	.name {
		font-weight: var(--font-weight-semibold);
	}
	.message[data-role='assistant'] .name {
		color: var(--color-role-assistant-text);
	}
	.message[data-role='user'] .name {
		color: var(--color-role-user-text);
	}
	.message[data-role='system'] .name {
		color: var(--color-role-system-text);
	}

	.actions {
		display: flex;
		flex-direction: row;
		gap: var(--spacing-xs);
	}

	.body {
		color: var(--color-text-primary);
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}

	.editor {
		resize: vertical;
	}
	.edit-actions {
		display: flex;
		flex-direction: row;
		gap: var(--spacing-sm);
	}

	.slides {
		display: flex;
		flex-direction: row;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.counter {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	/* Subtle, transparent icon button. Global `button:hover` supplies the hover
	   background + accent colour from standardControls.css. */
	.icon-btn {
		background-color: transparent;
		justify-content: center;
		align-items: center;
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.icon-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}
	.icon-btn :global(svg) {
		width: 16px;
		height: 16px;
	}

	.typing {
		display: inline-flex;
		gap: 4px;
		align-items: center;
		height: var(--line-height-normal);
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: var(--radius-full);
		background-color: var(--color-text-muted);
		animation: blink 1.2s var(--easing-default) infinite both;
	}
	.typing span:nth-child(2) {
		animation-delay: 0.2s;
	}
	.typing span:nth-child(3) {
		animation-delay: 0.4s;
	}
	@keyframes blink {
		0%,
		80%,
		100% {
			opacity: 0.2;
		}
		40% {
			opacity: 1;
		}
	}
</style>
