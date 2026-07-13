<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { ArrowLeft, Pause, Play, Square, StepForward } from '@lucide/svelte';
	import AgentRail from './AgentRail.svelte';
	import AgentInspector from './AgentInspector.svelte';
	import RealmTimeline from './RealmTimeline.svelte';
	import PlayerTurnBar from './PlayerTurnBar.svelte';

	const realm = $derived(realmStore.realm);
	const streaming = $derived(realmStore.isStreaming);

	/** Whose turn the round-robin policy will pick next. */
	const nextAgent = $derived.by(() => {
		const r = realmStore.realm;
		const agents = [...realmStore.agents].sort(
			// numeric-string compare for the snowflake tie-break
			(a, b) => a.turn_order - b.turn_order || a.id.length - b.id.length || a.id.localeCompare(b.id)
		);
		if (!r || agents.length === 0) return null;
		return agents[r.tick % agents.length];
	});
</script>

{#if realm}
	<div class="view">
		<header class="topbar">
			<button type="button" class="back" title="Back to realms" onclick={() => realmStore.closeRealm()}>
				<ArrowLeft size={16} />
			</button>
			<h2>{realm.name}</h2>
			<span class="tick">tick {realm.tick}</span>
			<span class="status" data-status={realm.status}>{realm.status}</span>

			<div class="controls">
				{#if streaming}
					<button type="button" onclick={() => realmStore.cancel()} title="Cancel the in-flight turn">
						<Square size={14} />
						Stop
					</button>
				{:else}
					<button
						type="button"
						onclick={() => realmStore.step()}
						disabled={realmStore.agents.length === 0}
						title="Run a single turn"
					>
						<StepForward size={14} />
						Step
					</button>
					<button
						type="button"
						class="primary"
						onclick={() => realmStore.play()}
						disabled={realmStore.agents.length === 0}
						title="Run turns until a player's turn comes up"
					>
						<Play size={14} />
						Run
					</button>
				{/if}
				{#if streaming && realmStore.autoRun}
					<button type="button" onclick={() => realmStore.pause()} title="Finish this turn, then stop">
						<Pause size={14} />
						Pause
					</button>
				{/if}
			</div>
		</header>

		<div class="body">
			<AgentRail nextAgentId={nextAgent?.id ?? null} />

			<main class="stage">
				<RealmTimeline />
				<PlayerTurnBar />
			</main>

			{#if realmStore.inspection}
				<AgentInspector inspection={realmStore.inspection} />
			{/if}
		</div>
	</div>
{/if}

<style>
	.view {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.topbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		backdrop-filter: var(--backdrop-blur);
	}

	.topbar h2 {
		margin: 0;
		font-size: var(--font-size-lg, 1.1rem);
	}

	.back {
		display: flex;
		align-items: center;
		background: none;
		border: none;
		color: var(--color-text-secondary);
		padding: var(--spacing-xs);
	}

	.tick {
		font-size: 0.8rem;
		color: var(--color-text-muted);
		border: 1px solid var(--color-border-subtle);
		border-radius: 999px;
		padding: 2px 10px;
	}

	.status {
		font-size: 0.8rem;
		color: var(--color-text-muted);
		text-transform: capitalize;
	}

	.status[data-status='active'] {
		color: var(--color-success, #4caf7d);
	}

	.controls {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
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
		box-shadow: var(--shadow-md);
	}

	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}

	.body {
		display: flex;
		flex: 1;
		min-height: 0;
	}

	.stage {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
	}
</style>
