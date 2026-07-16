<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import type { ActionSpec } from '$lib/types/realm.js';
	import { Eye, Pause, Play, Square, StepForward, User } from '@lucide/svelte';
	import PlayerTurnBar from '../realms/PlayerTurnBar.svelte';

	const api = useApi();

	const streaming = $derived(chatStore.isStreaming);
	const awaiting = $derived(chatStore.awaitingRealmPlayer);
	const nextAgent = $derived(chatStore.nextRealmAgent);
	const empty = $derived(chatStore.activePath.length === 0);
	// Only offer the vantage toggle when a player agent exists to have one.
	const canToggleView = $derived(chatStore.viewerAgent != null);

	let actions = $state<ActionSpec[]>([]);

	// The action space belongs to whoever the turn is waiting on.
	$effect(() => {
		const convId = chatStore.conversationId;
		const agentId = awaiting?.id ?? null;
		if (!convId || !agentId) {
			actions = [];
			return;
		}
		api.chat.realm.agents
			.actions(convId, agentId)
			.then((a) => (actions = a))
			.catch(() => (actions = []));
	});
</script>

{#if awaiting}
	{#key awaiting.id}
		<PlayerTurnBar
			agentName={awaiting.name}
			{actions}
			disabled={streaming}
			onsubmit={(content, action) => chatStore.submitRealmPlayerTurn(content, action)}
		/>
	{/key}
{:else}
	<div class="bar">
		<span class="status">
			{#if streaming}
				Running turn…
			{:else if empty}
				Fresh playthrough — run the first turn.
			{:else if nextAgent}
				Next to act: <strong>{nextAgent.name}</strong>
			{/if}
		</span>

		<div class="controls">
			{#if canToggleView}
				<button
					type="button"
					class="view-toggle"
					onclick={() => (chatStore.realmDirectorView = !chatStore.realmDirectorView)}
					title={chatStore.realmDirectorView
						? 'Showing every turn — click to play as the player'
						: "Showing the player's view — click to see every turn"}
				>
					{#if chatStore.realmDirectorView}
						<Eye size={14} /> Director
					{:else}
						<User size={14} /> Player
					{/if}
				</button>
			{/if}
			{#if streaming}
				{#if chatStore.realmAutoRun}
					<button type="button" onclick={() => chatStore.pauseRealm()} title="Finish this turn, then stop">
						<Pause size={14} />
						Pause
					</button>
				{/if}
				<button type="button" onclick={() => chatStore.cancelStream()} title="Cancel the in-flight turn">
					<Square size={14} />
					Stop
				</button>
			{:else}
				<button type="button" onclick={() => chatStore.runRealmTurn()} title="Run a single turn">
					<StepForward size={14} />
					Step
				</button>
				<button
					type="button"
					class="primary"
					onclick={() => chatStore.runRealmTurns()}
					title="Run turns until it's your turn"
				>
					<Play size={14} />
					Run
				</button>
			{/if}
		</div>
	</div>
{/if}

<style>
	.bar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-height: 4em;
		padding: var(--spacing-sm) var(--spacing-lg);
		border-top: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
	}

	.status {
		flex: 1;
		color: var(--color-text-secondary);
		font-size: 0.9rem;
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
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
</style>
