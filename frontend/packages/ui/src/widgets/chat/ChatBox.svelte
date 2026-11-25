<script lang="ts" module>
	let feedEl: HTMLDivElement | undefined;

	const ARROW_STEP = 40;

	export function scrollFeed(direction: 1 | -1) {
		feedEl?.scrollBy({ top: direction * ARROW_STEP });
	}
</script>

<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
	import MessageBubble from './MessageBubble.svelte';
	import ContextInspector from './ContextInspector.svelte';
	import SetupGuide, { loadSetupState, setupComplete, setupKnown } from './SetupGuide.svelte';
	import { onMount, tick } from 'svelte';

	onMount(() => {
		if (!userWidgetStore.loadedWidgets) void userWidgetStore.loadWidgets();
		if (!userWidgetStore.builtinWidgets) void userWidgetStore.loadBuiltins();
		loadSetupState();
		feedEl = scrollEl;
		return () => {
			if (feedEl === scrollEl) feedEl = undefined;
		};
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
	{#if feed.length === 0 && setupKnown() && !setupComplete()}
		<SetupGuide />
	{:else if feed.length === 0}
		<div class="empty">
			{#if chatStore.conversationId}
				{t.chat.feed.nothingYet}
			{:else}
				{t.chat.feed.pickCharacter}
			{/if}
		</div>
	{:else}
		{#each feed as entry, i (entry.message.id)}
			{#if entry.audible}
				<MessageBubble message={entry.message} last={i === feed.length - 1} />
			{/if}
			{#each entry.notices as notice, n (n)}
				<div class="dispatch">{notice}</div>
			{/each}
		{/each}
	{/if}

	{#if chatStore.hiddenTurnActive}
		<div class="stir">{t.chat.feed.worldStirs}</div>
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
	@media (max-width: 768px) {
		.container {
			gap: var(--spacing-md);
			padding: var(--spacing-sm);
		}
	}
</style>
