<script lang="ts">
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
			Running turn…
		{:else if awaiting}
			Your turn as <strong>{awaiting.name}</strong>
		{:else if empty}
			Nothing has happened yet — run the first turn.
		{:else if nextAgent}
			Next to act: <strong>{nextAgent.name}</strong>
		{:else}
			Nobody can act right now.
		{/if}
	</span>

	<div class="controls">
		{#if canToggleView}
			<button
				type="button"
				onclick={() => (chatStore.directorView = !chatStore.directorView)}
				{@attach tooltip(
					chatStore.directorView ? 'Seeing every turn' : 'Seeing only what your agent perceives'
				)}
			>
				{#if chatStore.directorView}
					<Eye size={14} /> Director
				{:else}
					<User size={14} /> Player
				{/if}
			</button>
		{/if}
		{#if streaming}
			{#if chatStore.autoRun}
				<button
					type="button"
					onclick={() => chatStore.pause()}
					{@attach tooltip('Finish this turn, then stop')}
				>
					<Pause size={14} /> Pause
				</button>
			{/if}
			<button
				type="button"
				onclick={() => chatStore.cancelStream()}
				{@attach tooltip('Cancel this turn')}
			>
				<Square size={14} /> Stop
			</button>
		{:else}
			<button
				type="button"
				disabled={awaiting != null}
				onclick={() => chatStore.step()}
				{@attach tooltip('Run a single turn')}
			>
				<StepForward size={14} /> Step
			</button>
			<button
				type="button"
				class="primary"
				disabled={awaiting != null}
				onclick={() => chatStore.play()}
				{@attach tooltip("Run turns until it's your turn")}
			>
				<Play size={14} /> Run
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
