<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import MessageBubble from './MessageBubble.svelte';
	import ContextInspector from './ContextInspector.svelte';
	import { tick } from 'svelte';

	let scrollEl: HTMLDivElement;
	let pinned = $state(true);
	const THRESHOLD = 10; // px from bottom that still counts as "at bottom"

	function onScroll() {
		if (!scrollEl) return;
		pinned = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < THRESHOLD;
	}

	$effect(() => {
		const path = chatStore.activePath;
		void path.length;
		void path.at(-1)?.content;
		if (!pinned) return;
		tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});
</script>

<div class="container" bind:this={scrollEl} onscroll={onScroll}>
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
