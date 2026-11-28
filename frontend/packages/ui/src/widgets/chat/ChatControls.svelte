<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { tick } from 'svelte';
	import CommandAutocomplete from './CommandAutocomplete.svelte';
	import { scrollFeed } from './ChatBox.svelte';
	import PopupMenu from '#lib/components/PopupMenu.svelte';
	import ImageAttachments from '#lib/components/ImageAttachments.svelte';
	import {
		completionAt,
		dispatch,
		resourceCatalog,
		resourceLabel,
		serverCommands,
		type Suggestion
	} from '#lib/commands/index.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { conversationManager } from '#lib/stores/conversationManager.svelte.js';
	import { pluginCalls } from '#lib/stores/pluginCalls.svelte.js';
	import { FilePlus, MessagesSquare, Paperclip, Send, Sparkles, Square, X } from '@lucide/svelte';
	import { CHAT_COMPOSER_LAYER, type ChatComposerLayer } from '#lib/keys/commands.js';
	import { keyLayer } from '#lib/keys/layers.svelte.js';
	import { inputEnvironment } from '#lib/keys/environment.svelte.js';

	let runningCommand = $derived(serverCommands.running);
	/** The plugin the user is waiting for, if any. */
	let waitingOn = $derived(pluginCalls.waitingOn(chatStore.conversationId));
	let busy = $derived(chatStore.isStreaming || runningCommand !== null || waitingOn !== null);

	let inputText = $state('');
	let textareaEl: HTMLTextAreaElement;
	let fileInput: HTMLInputElement;

	const IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'];
	let uploading = $derived(chatStore.uploadingImages > 0);

	function attach(files: FileList | null | undefined): boolean {
		const images = [...(files ?? [])].filter((f) => IMAGE_TYPES.includes(f.type));
		if (images.length > 0) void chatStore.attachImages(images);
		return images.length > 0;
	}

	function onpaste(event: ClipboardEvent) {
		if (attach(event.clipboardData?.files)) event.preventDefault();
	}

	function ondragover(event: DragEvent) {
		if (event.dataTransfer?.types.includes('Files')) event.preventDefault();
	}

	function ondrop(event: DragEvent) {
		if (!event.dataTransfer?.types.includes('Files')) return;
		event.preventDefault();
		attach(event.dataTransfer.files);
	}

	let activeCharacter = $derived(chatStore.characterId);

	/** Set by Escape; cleared as soon as the input changes again. */
	let dismissed = $state(false);
	let cursor = $state(0);

	function resize() {
		dismissed = false;
		syncCaret();
		textareaEl.style.height = 'auto';
		textareaEl.style.height = textareaEl.scrollHeight + 'px';
	}

	/** Where the caret is. */
	let caret = $state(0);

	function syncCaret() {
		caret = textareaEl?.selectionStart ?? inputText.length;
	}

	let completion = $derived(dismissed ? { kind: 'none' as const } : completionAt(inputText, caret));
	let suggestions = $derived<Suggestion[]>(
		completion.kind === 'none' ? [] : completion.suggestions
	);
	let heading = $derived(
		completion.kind === 'resource' ? resourceLabel(completion.resource) : undefined
	);
	let active = $derived(Math.min(cursor, Math.max(0, suggestions.length - 1)));

	// Listings load lazily, so asking for them is what starts the fetch.
	$effect(() => {
		if (completion.kind === 'resource') void resourceCatalog.prime(completion.resource);
	});

	function accept(suggestion: Suggestion) {
		if (completion.kind === 'none') return;
		const { from, to } = completion;
		inputText = inputText.slice(0, from) + suggestion.insert + inputText.slice(to);
		const at = from + suggestion.insert.length;
		cursor = 0;
		tick().then(() => {
			textareaEl?.focus();
			textareaEl?.setSelectionRange(at, at);
			syncCaret();
			resize();
		});
	}

	function send() {
		if (uploading) return;
		const text = inputText.trim();
		if (!text) {
			if (chatStore.queuedActions.length > 0 || chatStore.queuedImages.length > 0) {
				void chatStore.sendMessage('');
			}
			return;
		}
		inputText = '';
		dismissed = false;
		cursor = 0;
		textareaEl.style.height = 'auto';
		void dispatch(text);
	}

	function handleMessageSend(event: KeyboardEvent) {
		if (suggestions.length > 0) {
			switch (event.key) {
				case 'ArrowDown':
					event.preventDefault();
					cursor = (active + 1) % suggestions.length;
					return;
				case 'ArrowUp':
					event.preventDefault();
					cursor = (active - 1 + suggestions.length) % suggestions.length;
					return;
				case 'Tab':
				case 'Enter':
					event.preventDefault();
					accept(suggestions[active]);
					return;
				case 'Escape':
					event.preventDefault();
					dismissed = true;
					return;
			}
		}
		if (
			inputText === '' &&
			(event.key === 'ArrowUp' || event.key === 'ArrowDown') &&
			!(event.altKey || event.ctrlKey || event.metaKey || event.shiftKey)
		) {
			event.preventDefault();
			scrollFeed(event.key === 'ArrowUp' ? -1 : 1);
			return;
		}
		if (event.key === 'Enter' && !event.shiftKey && !inputEnvironment.touchOnly) {
			event.preventDefault();
			send();
		}
	}

	const composerLayer: ChatComposerLayer = {
		focus: () => textareaEl?.focus(),
		send,
		draft: (content, mode) => {
			inputText = mode === 'append' ? inputText + content : content;
			dismissed = false;
			cursor = 0;
			caret = inputText.length;
			// Ensure the text area resizes accordingly
			tick().then(() => {
				textareaEl?.focus();
				if (textareaEl) resize();
			});
		}
	};

	async function startNewChat() {
		if (activeCharacter) {
			const character = characterStore.activeCharacter;
			if (character) {
				chatStore.beginNewConversation({
					characterId: character.id,
					versionId: character.current_version_id,
					greetings: character.definition.greetings
				});
			}
		}
	}

	async function cancelGeneration() {
		if (runningCommand) {
			await serverCommands.cancel();
			return;
		}
		if (waitingOn) {
			await pluginCalls.cancel(chatStore.conversationId);
			return;
		}
		if (chatStore.isStreaming && !chatStore.isCancelling) {
			await chatStore.cancelStream();
		}
	}
</script>

<div class="container" role="group" {ondragover} {ondrop}>
	{#if chatStore.queuedImages.length > 0 || uploading}
		<div class="queued">
			<ImageAttachments
				uris={chatStore.queuedImages}
				onremove={(uri) => chatStore.unqueueImage(uri)}
			/>
			{#if uploading}
				<span class="queued-action">{t.chat.composer.uploadingImage}</span>
			{/if}
		</div>
	{/if}
	{#if chatStore.queuedActions.length > 0}
		<div class="queued">
			{#each chatStore.queuedActions as queued (queued.key)}
				<span class="queued-action">
					{queued.label}
					<button
						type="button"
						aria-label={t.chat.composer.removeAction({ label: queued.label })}
						onclick={() => chatStore.unqueueAction(queued.key)}
					>
						<X size={11} />
					</button>
				</span>
			{/each}
		</div>
	{/if}
	<div class="control-container">
		{#if suggestions.length > 0}
			<CommandAutocomplete items={suggestions} activeIndex={active} {heading} onSelect={accept} />
		{/if}
		<PopupMenu
			title={t.chat.composer.functions}
			items={[
				{
					label: t.chat.composer.conversations,
					icon: MessagesSquare,
					onSelect: () => {
						if (activeCharacter) conversationManager.open(activeCharacter);
					},
					disabled: activeCharacter === null
				},
				{
					label: t.chat.composer.newChat,
					icon: FilePlus,
					onSelect: async () => await startNewChat(),
					disabled: activeCharacter === null
				}
			]}
		>
			{#snippet icon()}<Sparkles />{/snippet}
		</PopupMenu>
		<button
			class="attachement-button"
			{@attach tooltip(t.chat.composer.attachments)}
			aria-label={t.chat.composer.attachments}
			onclick={() => fileInput.click()}><Paperclip /></button
		>
		<input
			bind:this={fileInput}
			type="file"
			accept={IMAGE_TYPES.join(',')}
			multiple
			hidden
			style="display:none"
			onchange={() => {
				attach(fileInput.files);
				fileInput.value = '';
			}}
		/>
		<textarea
			use:keyLayer={{ id: CHAT_COMPOSER_LAYER, data: composerLayer }}
			class="text-input"
			placeholder={t.chat.composer.placeholder}
			rows="1"
			bind:value={inputText}
			bind:this={textareaEl}
			onkeydown={handleMessageSend}
			onkeyup={syncCaret}
			onclick={syncCaret}
			oninput={resize}
			{onpaste}
		></textarea>
		{#if !busy}
			<button
				class="send-button"
				aria-label={t.chat.composer.send}
				disabled={uploading}
				onclick={send}><Send /></button
			>
		{:else}
			<button
				class="send-button"
				{@attach tooltip(
					runningCommand
						? t.chat.composer.stopCommand({ name: runningCommand.name })
						: waitingOn
							? t.chat.composer.stopPlugin({ name: waitingOn })
							: t.chat.composer.stopGenerating
				)}
				onclick={cancelGeneration}><Square /></button
			>
		{/if}
	</div>
</div>

<style>
	.container {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--spacing-xs);
		min-height: 4em;
		padding: var(--spacing-xs) var(--spacing-md);
		background-color: var(--color-bg-base);
	}
	.queued {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--spacing-xs);
	}
	.queued :global(.open) {
		width: 3.5rem;
		height: 3.5rem;
	}
	.queued-action {
		display: inline-flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: 2px var(--spacing-sm);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-elevated);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
	}
	.queued-action button {
		display: flex;
		padding: 0;
		background: none;
		border: none;
		color: inherit;
	}
	.queued-action button:hover {
		color: var(--color-text-accent);
	}
	.control-container {
		position: relative; /* anchors the command autocomplete */
		display: flex;
		flex-direction: row;
		background-color: var(--color-bg-elevated);
		border-radius: var(--radius-md);
		width: 100%;
		border: 1px solid var(--color-border-subtle);
	}
	.attachement-button,
	.send-button {
		justify-content: center;
		align-items: center;
	}
	.text-input {
		display: flex;
		flex: 1;
		width: 100%;
		font-size: var(--font-size-md);
		resize: none;
		overflow-y: auto;
		max-height: 8em;
		margin: 0;
		padding: calc(var(--spacing-sm) + 4px) var(--spacing-sm);
	}
	@media (max-width: 768px) {
		.container {
			padding: 0 var(--spacing-sm);
		}
	}
</style>
