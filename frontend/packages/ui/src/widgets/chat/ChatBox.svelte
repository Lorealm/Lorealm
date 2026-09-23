<script lang="ts">
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
	import MessageBubble from './MessageBubble.svelte';
	import ContextInspector from './ContextInspector.svelte';
	import { onMount, tick } from 'svelte';

	// Loaded once here rather than per bubble; every message renders the
	// viewer's static widgets from this list.
	onMount(() => {
		if (!userWidgetStore.loadedWidgets) void userWidgetStore.loadWidgets();
	});

	let scrollEl: HTMLDivElement;
	let pinned = $state(true);
	const THRESHOLD = 10; // px from bottom that still counts as "at bottom"

	function onScroll() {
		if (!scrollEl) return;
		pinned = scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight < THRESHOLD;
	}

	const feed = $derived(chatStore.renderedFeed);

	$effect(() => {
		void feed.length;
		void feed.at(-1)?.message.content;
		void chatStore.hiddenTurnActive;
		if (!pinned) return;
		tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});
</script>

<div class="container" bind:this={scrollEl} onscroll={onScroll}>
	{#if feed.length === 0}
		<div class="empty">
			{#if chatStore.isRealm}
				A fresh playthrough — run the first turn below.
			{:else}
				Pick a character and send a message to begin.
			{/if}
		</div>
	{:else}
		{#each feed as entry, i (entry.message.id)}
			{#if entry.audible}
				<MessageBubble message={entry.message} last={i === feed.length - 1} />
			{/if}
			{#each entry.observations as observation (observation.id)}
				<div class="dispatch">{observation.content}</div>
			{/each}
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
	.dispatch {
		align-self: stretch;
		white-space: pre-wrap;
		color: var(--color-role-system-text);
		background: var(--color-role-system-bg);
		border: 1px solid var(--color-role-system);
		border-radius: var(--radius-md);
		padding: var(--spacing-sm) var(--spacing-md);
		font-size: var(--font-size-sm);
		line-height: var(--line-height-normal);
	}
	.stir {
		align-self: center;
		color: var(--color-text-muted);
		font-style: italic;
		font-size: 0.9rem;
		padding: var(--spacing-sm);
	}
</style>
