<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { chatStore } from '#lib/stores/chat.svelte.js';
	import { Eye, Pause, Play, Square, StepForward, User } from '@lucide/svelte';

	const streaming = $derived(chatStore.isStreaming);
	const awaiting = $derived(chatStore.awaitingPlayer);
	const nextAgent = $derived(chatStore.nextAgent);
	const empty = $derived(chatStore.activePath.length === 0);
	const canToggleView = $derived(chatStore.viewerAgent != null);
</script>

<div class="bar">
	<span class="status">
		{#if streaming}
			{t.chat.turns.running}
		{:else if awaiting}
			{t.chat.turns.yourTurnAs} <strong>{awaiting.name}</strong>
		{:else if empty}
			{t.chat.turns.emptyRunFirst}
		{:else if nextAgent}
			{t.chat.turns.nextToAct} <strong>{nextAgent.name}</strong>
		{:else}
			{t.chat.turns.nobody}
		{/if}
	</span>

	<div class="controls">
		{#if canToggleView}
			<button
				type="button"
				onclick={() => (chatStore.directorView = !chatStore.directorView)}
				{@attach tooltip(chatStore.directorView ? t.chat.turns.seeingAll : t.chat.turns.seeingOwn)}
			>
				{#if chatStore.directorView}
					<Eye size={14} /> {t.chat.turns.director}
				{:else}
					<User size={14} /> {t.chat.turns.player}
				{/if}
			</button>
		{/if}
		{#if streaming}
			{#if chatStore.autoRun}
				<button
					type="button"
					onclick={() => chatStore.pause()}
					{@attach tooltip(t.chat.turns.pauseHint)}
				>
					<Pause size={14} />
					{t.chat.turns.pause}
				</button>
			{/if}
			<button
				type="button"
				onclick={() => chatStore.cancelStream()}
				{@attach tooltip(t.chat.turns.cancelHint)}
			>
				<Square size={14} />
				{t.chat.turns.stop}
			</button>
		{:else}
			<button
				type="button"
				disabled={awaiting != null}
				onclick={() => chatStore.step()}
				{@attach tooltip(t.chat.turns.stepHint)}
			>
				<StepForward size={14} />
				{t.chat.turns.step}
			</button>
			<button
				type="button"
				class="primary"
				disabled={awaiting != null}
				onclick={() => chatStore.play()}
				{@attach tooltip(t.chat.turns.runHint)}
			>
				<Play size={14} />
				{t.chat.turns.run}
			</button>
		{/if}
	</div>
</div>

<style>
	.bar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
	}
	.status {
		flex: 1;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.controls {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.controls button {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		font-size: var(--font-size-sm);
	}
	.controls button:disabled {
		opacity: 0.5;
	}
	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
</style>
