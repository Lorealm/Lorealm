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

	// In a realm, render the playthrough from the player's vantage
	const path = $derived(chatStore.isRealm ? chatStore.visiblePath : chatStore.activePath);

	$effect(() => {
		void path.length;
		void path.at(-1)?.content;
		void chatStore.hiddenTurnActive;
		if (!pinned) return;
		tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});
</script>

<div class="container" bind:this={scrollEl} onscroll={onScroll}>
	{#if path.length === 0}
		<div class="empty">
			{#if chatStore.isRealm}
				A fresh playthrough — run the first turn below.
			{:else}
				Pick a character and send a message to begin.
			{/if}
		</div>
	{:else}
		{#each path as message, i (message.id)}
			<MessageBubble {message} last={i === path.length - 1} />
		{/each}
	{/if}

	{#if chatStore.hiddenTurnActive}
		<div class="stir">the world stirs…</div>
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
	.stir {
		align-self: center;
		color: var(--color-text-muted);
		font-style: italic;
		font-size: 0.9rem;
		padding: var(--spacing-sm);
	}
</style>
