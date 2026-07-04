<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import type { ConversationMessage } from '$lib/types/conversation.js';
	import TransportImage from '$lib/components/TransportImage.svelte';
	import ChatWidget from '$lib/components/ChatWidget.svelte';
	import { ChevronLeft, ChevronRight, Layers, Pencil, Trash2 } from '@lucide/svelte';
	import { contextInspector } from '$lib/stores/contextInspector.svelte.js';
	import { parseContextBlocks } from '$lib/types/context.js';
	import { parseReasoning, parseWidgets, splitWidgetMarkers } from '$lib/types/widgets.js';
	import { marked } from '$lib/assets/scripts/marked-rp.js';
	import DOMPurify from 'dompurify';

	let { message, last = false }: { message: ConversationMessage; last?: boolean } = $props();

	let info = $derived(chatStore.siblingInfo(message.id));
	let name = $derived(
		message.role === 'user'
			? 'You'
			: message.role === 'system'
				? 'System'
				: (characterStore.activeCharacter?.definition.name ?? 'Assistant')
	);

	let committed = $derived(!message.id.startsWith('tmp_'));
	let typing = $derived(last && chatStore.isStreaming && message.content.length === 0);
	let busy = $derived(chatStore.isStreaming);

	let editing = $state(false);
	let editMode = $state<'edit' | 'branch'>('edit');
	let editText = $state('');

	let hasContext = $derived(parseContextBlocks(message.metadata) !== null);
	let reasoning = $derived(parseReasoning(message.metadata));
	let widgets = $derived(parseWidgets(message.metadata));
	let segments = $derived(splitWidgetMarkers(message.content));
	let trailing = $derived(
		widgets.filter((_, i) => !segments.some((s) => s.type === 'widget' && s.index === i))
	);

	const renderMd = (text: string) => DOMPurify.sanitize(marked.parse(text).toString().trim());

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

	function onRight() {
		if (info.index < info.count - 1) {
			chatStore.swipeNext(message.id);
		} else if (message.role === 'assistant') {
			chatStore.swipeNext(message.id);
		} else {
			startEdit('branch');
		}
	}

	function autosize(node: HTMLTextAreaElement) {
		const resize = () => {
			node.style.height = 'auto';
			node.style.height = `${node.scrollHeight + node.offsetHeight - node.clientHeight}px`;
		};
		resize();
		node.addEventListener('input', resize);
		return {
			destroy() {
				node.removeEventListener('input', resize);
			}
		};
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

		{#if !editing && reasoning.length > 0}
			<details class="reasoning" open={false}>
				<summary>{committed ? 'Thoughts' : 'Thinking…'}</summary>
				<div class="reasoning-body">{reasoning}</div>
			</details>
		{/if}

		{#if editing}
			<textarea class="editor" bind:value={editText} use:autosize></textarea>
			<div class="edit-actions">
				<button disabled={busy} onclick={saveEdit}>{editMode === 'edit' ? 'Save' : 'Swipe'}</button>
				<button onclick={() => (editing = false)}>Cancel</button>
			</div>
		{:else if typing}
			<div class="body">
				<!-- YUMMYYYYYY (this is a trick for the animated message indicator btw. `<span />` is apparently illegal) -->
				<span class="typing"><span></span><span></span><span></span></span>
			</div>
		{:else}
			<div class="body md">
				{#each segments as seg, i (i)}
					{#if seg.type === 'text'}
						<!-- eslint-disable-next-line svelte/no-at-html-tags -->
						{@html renderMd(seg.text)}
					{:else if widgets[seg.index]}
						<div class="inline-widget">
							<ChatWidget widget={widgets[seg.index]} />
						</div>
					{/if}
				{/each}
			</div>
		{/if}

		{#if !editing && trailing.length > 0}
			<div class="widgets">
				{#each trailing as widget, i (i)}
					<ChatWidget {widget} />
				{/each}
			</div>
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

	.widgets {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	.reasoning {
		border-left: 2px solid var(--color-border-medium);
		padding-left: var(--spacing-sm);
	}
	.reasoning summary {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		cursor: pointer;
		user-select: none;
	}
	.reasoning-body {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		font-style: italic;
		line-height: var(--line-height-normal);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		padding-top: var(--spacing-xs);
	}

	.inline-widget {
		margin: var(--spacing-sm) 0;
	}

	.editor {
		width: 100%;
		box-sizing: border-box;
		min-height: 4.5em;
		padding: var(--spacing-sm);
		color: var(--color-text-primary);
		background-color: var(--color-bg-base);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		font-family: var(--font-family-nunito);
		font-size: var(--font-size-md);
		line-height: var(--line-height-normal);
		resize: none;
		overflow-y: hidden;
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

	.md :global {
		img {
			max-width: 100%;
			height: auto;
		}
		em {
			font-style: italic;
			color: var(--color-text-muted);
		}
		em .quoted {
			color: var(--color-primary-dark);
		}
		.quoted {
			color: var(--color-primary-light);
		}
		.quoted > em {
			color: var(--color-primary-dark);
		}
		p {
			margin: 0;
			padding: 0;
		}
		ol {
			gap: 0;
			margin: 0;
			padding-top: 0;
			padding-bottom: 0;
			line-height: 1em;
		}
		li {
			margin: 0;
			padding: 0;
		}
		details {
			margin: 0;
		}
		summary {
			margin: 0;
			line-height: 1em;
		}

		code {
			display: inline-block;
			background-color: var(--color-bg-base);
			padding: var(--spacing-xs) var(--spacing-sm);
			border-radius: var(--radius-sm);
			font-family: var(--font-family-mono);
			font-size: var(--font-size-sm);
		}

		pre code {
			display: block;
			padding: var(--spacing-md);
			width: 100%;
			border-radius: var(--radius-md);
			font-size: var(--font-size-base);
			white-space: pre;
			overflow-x: auto;
		}
		font-family: var(--font-family-nunito);
		font-size: var(--font-size-md);
	}
</style>
