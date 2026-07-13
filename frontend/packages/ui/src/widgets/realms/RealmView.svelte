<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { ArrowLeft, Pause, Play, Square, StepForward, Swords } from '@lucide/svelte';
	import AgentRail from './AgentRail.svelte';
	import AgentInspector from './AgentInspector.svelte';
	import RealmTimeline from './RealmTimeline.svelte';
	import PlayerTurnBar from './PlayerTurnBar.svelte';

	const realm = $derived(realmStore.realm);
	const playing = $derived(realmStore.inPlaythrough);
	const streaming = $derived(realmStore.isStreaming);

	let starting = $state(false);

	async function startConversation() {
		if (starting) return;
		starting = true;
		try {
			await realmStore.startPlaythrough();
		} finally {
			starting = false;
		}
	}
</script>

{#if realm}
	<div class="view">
		<header class="topbar">
			{#if playing}
				<button
					type="button"
					class="back"
					title="Leave this playthrough (it stays in your conversations)"
					onclick={() => realmStore.exitPlaythrough()}
				>
					<ArrowLeft size={16} />
				</button>
				<h2>{realm.definition.name}</h2>
				<span class="pill">turn {realmStore.turnCount}</span>

				<div class="controls">
					{#if streaming}
						<button type="button" onclick={() => realmStore.cancel()} title="Cancel the in-flight turn">
							<Square size={14} />
							Stop
						</button>
					{:else}
						<button type="button" onclick={() => realmStore.step()} title="Run a single turn">
							<StepForward size={14} />
							Step
						</button>
						<button
							type="button"
							class="primary"
							onclick={() => realmStore.play()}
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
			{:else}
				<button type="button" class="back" title="Back to realms" onclick={() => realmStore.closeRealm()}>
					<ArrowLeft size={16} />
				</button>
				<h2>{realm.definition.name}</h2>
				{#each realm.tags as tag (tag)}
					<span class="pill">{tag}</span>
				{/each}

				<div class="controls">
					<button
						type="button"
						class="primary"
						onclick={startConversation}
						disabled={starting || realmStore.agents.length === 0}
						title={realmStore.agents.length === 0
							? 'Add at least one agent first'
							: 'Instantiate this realm into a fresh conversation'}
					>
						<Swords size={14} />
						{starting ? 'Starting…' : 'Start conversation'}
					</button>
				</div>
			{/if}
		</header>

		<div class="body">
			<AgentRail />

			<main class="stage">
				{#if playing}
					<RealmTimeline />
					<PlayerTurnBar />
				{:else}
					<div class="editorStage">
						{#if realm.definition.description}
							<p class="description">{realm.definition.description}</p>
						{/if}
						<div class="hint">
							This is the preset. Build the cast on the left — grants decide what each
							agent can know, bound tools decide what it can do. When you're ready,
							<strong>Start conversation</strong> boots the realm into its own playthrough:
							a private timeline that preset edits can never touch.
						</div>
					</div>
				{/if}
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

	.pill {
		font-size: 0.8rem;
		color: var(--color-text-muted);
		border: 1px solid var(--color-border-subtle);
		border-radius: 999px;
		padding: 2px 10px;
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

	.editorStage {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-md);
		padding: var(--spacing-4xl) var(--spacing-lg);
	}

	.description {
		max-width: 560px;
		margin: 0;
		color: var(--color-text-secondary);
		text-align: center;
		white-space: pre-wrap;
	}

	.hint {
		max-width: 560px;
		color: var(--color-text-muted);
		text-align: center;
		line-height: 1.5;
	}
</style>
