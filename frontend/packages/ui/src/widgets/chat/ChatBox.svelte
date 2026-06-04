<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import MessageBubble from './MessageBubble.svelte';
	import ContextInspector from './ContextInspector.svelte';
	import { tick } from 'svelte';

	let scrollEl: HTMLDivElement;

	// Stick to the bottom as the path grows and as tokens stream in. Reading
	// `activePath` (and the last bubble's content) subscribes us to both the
	// tree changing and the live stream buffer.
	$effect(() => {
		const path = chatStore.activePath;
		void path.length;
		void path.at(-1)?.content;
		tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});
</script>

<div class="container" bind:this={scrollEl}>
	{#if chatStore.activePath.length === 0}
		<div class="empty">Pick a character and send a message to begin.</div>
	{:else}
		{#each chatStore.activePath as message, i (message.id)}
			<MessageBubble {message} last={i === chatStore.activePath.length - 1} />
		{/each}
	{/if}
</div>

<ContextInspector />

<style>
	.container {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
	}
	.empty {
		margin: auto;
		color: var(--color-text-muted);
	}
</style>
