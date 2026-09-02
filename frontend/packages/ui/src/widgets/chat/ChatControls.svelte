<script lang="ts">
	import { tick } from 'svelte';
	import CommandAutocomplete from './CommandAutocomplete.svelte';
	import PopupMenu from '$lib/components/PopupMenu.svelte';
	import {
		dispatch,
		serverCommands,
		slashCommands,
		type RegisteredSlashCommand
	} from '$lib/commands/index.js';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { conversationManager } from '$lib/stores/conversationManager.svelte.js';
	import { FilePlus, MessagesSquare, Paperclip, Send, Sparkles, Square } from '@lucide/svelte';
	import { CHAT_COMPOSER_LAYER, type ChatComposerLayer } from '$lib/keys/commands.js';
	import { keyLayer } from '$lib/keys/layers.svelte.js';

	let runningCommand = $derived(serverCommands.running);
	let busy = $derived(chatStore.isStreaming || runningCommand !== null);

	let inputText = $state('');
	let textareaEl: HTMLTextAreaElement;

	let activeCharacter = $derived(chatStore.characterId);

	/** Set by Escape; cleared as soon as the input changes again. */
	let dismissed = $state(false);
	let cursor = $state(0);

	function resize() {
		dismissed = false;
		textareaEl.style.height = 'auto';
		textareaEl.style.height = textareaEl.scrollHeight + 'px';
	}

	// The popup is only up while the whole input is a bare command name.
	let suggestions = $derived.by(() => {
		if (dismissed) return [];
		const typing = /^\/([A-Za-z_-]*)$/.exec(inputText);
		return typing ? slashCommands.matching(typing[1]) : [];
	});
	let active = $derived(Math.min(cursor, Math.max(0, suggestions.length - 1)));

	function accept(command: RegisteredSlashCommand) {
		inputText = `/${command.name} `;
		cursor = 0;
		textareaEl?.focus();
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
		if (event.key === 'Enter' && !event.shiftKey) {
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
		if (chatStore.isStreaming && !chatStore.isCancelling) {
			await chatStore.cancelStream();
		}
	}
</script>

<div class="container">
	<div class="control-container">
		{#if suggestions.length > 0}
			<CommandAutocomplete items={suggestions} activeIndex={active} onSelect={accept} />
		{/if}
		<PopupMenu
			title="Functions"
			items={[
				{
					label: 'Conversations',
					icon: MessagesSquare,
					onSelect: () => {
						if (activeCharacter) conversationManager.open(activeCharacter);
					},
					disabled: activeCharacter === null
				},
				{
					label: 'Start new chat',
					icon: FilePlus,
					onSelect: async () => await startNewChat(),
					disabled: activeCharacter === null
				}
			]}
		>
			{#snippet icon()}<Sparkles />{/snippet}
		</PopupMenu>
		<button class="attachement-button" title="Attachments"><Paperclip /></button>
		<textarea
			use:keyLayer={{ id: CHAT_COMPOSER_LAYER, data: composerLayer }}
			class="text-input"
			placeholder="Send a message..."
			rows="1"
			bind:value={inputText}
			bind:this={textareaEl}
			onkeydown={handleMessageSend}
			oninput={resize}
		></textarea>
		{#if !busy}
			<button class="send-button" onclick={send}><Send /></button>
		{:else}
			<button
				class="send-button"
				title={runningCommand ? `Stop /${runningCommand.name}` : 'Stop generating'}
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
</style>
