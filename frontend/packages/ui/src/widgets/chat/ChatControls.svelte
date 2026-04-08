<script lang="ts">
	import { Paperclip, Send } from '@lucide/svelte';

	let inputText = $state('');
	let textareaEl: HTMLTextAreaElement;

	function resize() {
		textareaEl.style.height = 'auto';
		textareaEl.style.height = textareaEl.scrollHeight + 'px';
	}

	function handleMessageSend(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();

			inputText = '';
		}
	}
</script>

<div class="container">
	<div class="control-container">
		<button class="attachement-button"><Paperclip /></button>
		<textarea
			class="text-input"
			placeholder="Send a message..."
			rows="1"
			bind:value={inputText}
			bind:this={textareaEl}
			onkeydown={handleMessageSend}
			oninput={resize}
		></textarea>
		<button class="emoji-button"><Send /></button>
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
	.emoji-button {
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
