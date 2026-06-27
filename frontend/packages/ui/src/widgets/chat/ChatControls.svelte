<script lang="ts">
	import PopupMenu from '$lib/components/PopupMenu.svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { messageStateStore } from '$lib/stores/messageState.svelte.js';
	import { FilePlus, ListStart, Paperclip, Send, Sparkles, Square } from '@lucide/svelte';

	chatStore.ensureApi();
	messageStateStore.ensureApi();

	let busy = $derived(chatStore.isStreaming);

	let inputText = $state('');
	let textareaEl: HTMLTextAreaElement;

	let activeCharacter = $derived(chatStore.characterId);

	function resize() {
		textareaEl.style.height = 'auto';
		textareaEl.style.height = textareaEl.scrollHeight + 'px';
	}

	function send() {
		const text = inputText.trim();
		if (!text || chatStore.isStreaming) return;
		inputText = '';
		textareaEl.style.height = 'auto';
		chatStore.sendMessage(text).then(() => {
			const lastMsg = chatStore.activePath[-1];
			console.log('last message: ', lastMsg);
			messageStateStore.inspect(lastMsg.id, lastMsg.conversation_id);
		});
	}

	function handleMessageSend(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			send();
		}
	}

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
		if (chatStore.isStreaming && !chatStore.isCancelling) {
			await chatStore.cancelStream();
		}
	}
</script>

<div class="container">
	<div class="control-container">
		<PopupMenu
			title="Functions"
			items={[
				{ label: 'Run function', icon: ListStart, onSelect: () => console.log('function') },
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
			<button class="send-button" onclick={cancelGeneration}><Square /></button>
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
