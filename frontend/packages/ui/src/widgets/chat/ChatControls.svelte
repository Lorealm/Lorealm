<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { tick } from 'svelte';
	import CommandAutocomplete from './CommandAutocomplete.svelte';
	import PopupMenu from '#lib/components/PopupMenu.svelte';
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
	import { FilePlus, MessagesSquare, Paperclip, Send, Sparkles, Square } from '@lucide/svelte';
	import { CHAT_COMPOSER_LAYER, type ChatComposerLayer } from '#lib/keys/commands.js';
	import { keyLayer } from '#lib/keys/layers.svelte.js';
	import { inputEnvironment } from '#lib/keys/environment.svelte.js';

	let runningCommand = $derived(serverCommands.running);
	/** The plugin the user is waiting for, if any. */
	let waitingOn = $derived(pluginCalls.waitingOn(chatStore.conversationId));
	let busy = $derived(chatStore.isStreaming || runningCommand !== null || waitingOn !== null);

	let inputText = $state('');
	let textareaEl: HTMLTextAreaElement;

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
		const text = inputText.trim();
		if (!text) return;
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

<div class="container">
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
		<button class="attachement-button" {@attach tooltip(t.chat.composer.attachments)}
			><Paperclip /></button
		>
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
		></textarea>
		{#if !busy}
			<button class="send-button" aria-label={t.chat.composer.send} onclick={send}><Send /></button>
		{:else}
			<button
				class="send-button"
				{@attach tooltip(
					runningCommand
						? t.chat.composer.stopCommand(runningCommand.name)
						: waitingOn
							? t.chat.composer.stopPlugin(waitingOn)
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
		flex-direction: row;
		min-height: 4em;
		max-height: 10em;
		padding: 0 var(--spacing-md);
		background-color: var(--color-bg-base);
		align-items: center;
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
