<script lang="ts">
	import { useApi } from '$lib/api/client.js';
	import { chatStore } from '$lib/stores/chat.svelte.js';
	import { navigationStore, SidebarView } from '$lib/stores/navigation.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import type { RealmAgent } from '$lib/types/realm.js';
	import { ArrowLeft, FlaskConical, Pause, Play, Square, StepForward, Swords } from '@lucide/svelte';
	import AgentRail from './AgentRail.svelte';
	import RealmTimeline from './RealmTimeline.svelte';
	import PlayerTurnBar from './PlayerTurnBar.svelte';
	import AgentEditor from './AgentEditor.svelte';
	import ImportsPanel from './ImportsPanel.svelte';
	import RealmMetaForm from './RealmMetaForm.svelte';
	import RealmConfigPanel from './RealmConfigPanel.svelte';
	import VersionHistory from './VersionHistory.svelte';

	const api = useApi();

	const realm = $derived(realmStore.realm);
	const playing = $derived(realmStore.inPlaythrough);
	const streaming = $derived(realmStore.isStreaming);

	type EditorTab = 'overview' | 'cast';
	let editorTab = $state<EditorTab>('overview');
	/** The Cast tab's open editor: an agent, 'new', or nothing. */
	let editing = $state<RealmAgent | 'new' | null>(null);

	let starting = $state(false);

	const awaitingPlayer = $derived(
		realmStore.awaitingAgent?.kind === 'player' ? realmStore.awaitingAgent : null
	);

	// The action space belongs to whoever the turn is waiting on.
	$effect(() => {
		void realmStore.awaitingAgentId;
		realmStore.loadPlayerActions();
	});

	// Deleting the agent that's open in the editor closes the editor.
	$effect(() => {
		if (editing !== null && editing !== 'new') {
			const id = editing.id;
			if (!realmStore.agents.some((a) => a.id === id)) editing = null;
		}
	});

	/** A throwaway debug playthrough inside this tab. */
	async function startTestRun() {
		if (starting) return;
		starting = true;
		try {
			await realmStore.startPlaythrough();
		} finally {
			starting = false;
		}
	}

	/** Instantiate the realm and play it where chat lives: the Write tab. */
	async function playInWrite() {
		if (!realm || starting) return;
		starting = true;
		try {
			const full = await api.chat.realm.start({ realm_id: realm.id });
			await chatStore.loadConversation(full.metadata.id);
			navigationStore.setSidebarView(SidebarView.Write);
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), {
				title: 'Failed to start playthrough',
				durationMs: 5000
			});
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

				<nav class="tabs">
					<button
						type="button"
						class:active={editorTab === 'overview'}
						onclick={() => (editorTab = 'overview')}
					>
						Overview
					</button>
					<button
						type="button"
						class:active={editorTab === 'cast'}
						onclick={() => (editorTab = 'cast')}
					>
						Cast
					</button>
				</nav>

				<div class="controls">
					<button
						type="button"
						onclick={startTestRun}
						disabled={starting || realmStore.agents.length === 0}
						title={realmStore.agents.length === 0
							? 'Add at least one agent first'
							: 'Debug playthrough in this tab: step turns, inspect agents'}
					>
						<FlaskConical size={14} />
						Test run
					</button>
					<button
						type="button"
						class="primary"
						onclick={playInWrite}
						disabled={starting || realmStore.agents.length === 0}
						title={realmStore.agents.length === 0
							? 'Add at least one agent first'
							: 'Instantiate this realm and play it in the Write tab'}
					>
						<Swords size={14} />
						{starting ? 'Starting…' : 'Play'}
					</button>
				</div>
			{/if}
		</header>

		<div class="body">
			{#if playing}
				<AgentRail />
				<main class="stage">
					<RealmTimeline />
					{#if awaitingPlayer}
						{#key awaitingPlayer.id}
							<PlayerTurnBar
								agentName={awaitingPlayer.name}
								actions={realmStore.playerActions}
								disabled={streaming}
								onsubmit={(content, action) => realmStore.submitPlayerTurn(content, action)}
							/>
						{/key}
					{/if}
				</main>
			{:else if editorTab === 'overview'}
				<main class="stage">
					<div class="overviewStage">
						<div class="hint">
							This is the preset. <strong>Imports</strong> decide which of your
							resources exist inside the realm; the <strong>Cast</strong> tab builds
							the agents that use them. <strong>Test run</strong> plays a debug
							playthrough here; <strong>Play</strong> starts one in the Write tab.
							Playthroughs pin the cast, so preset edits never touch them.
						</div>
						<RealmMetaForm />
						<ImportsPanel />
						<RealmConfigPanel />
						<VersionHistory />
					</div>
				</main>
			{:else}
				<AgentRail
					selectedId={editing !== null && editing !== 'new' ? editing.id : null}
					onselect={(agent) => (editing = agent)}
					oncreate={() => (editing = 'new')}
				/>
				<main class="stage">
					<div class="castStage">
						{#if editing !== null}
							{#key editing === 'new' ? 'new' : editing.id}
								<AgentEditor
									agent={editing === 'new' ? null : editing}
									ondone={(saved) => (editing = saved)}
									oncancel={() => (editing = null)}
								/>
							{/key}
						{:else}
							<div class="hint centered">
								Pick an agent on the left to edit it, or add a new one. Grants decide
								what it can know, tools what it can do, lorebooks what lore feeds its
								context — the inspector on the right shows the resulting shape.
							</div>
						{/if}
					</div>
				</main>
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

	.tabs {
		display: flex;
		gap: 2px;
		margin-left: var(--spacing-lg);
	}

	.tabs button {
		font-size: 0.85rem;
		padding: var(--spacing-xs) var(--spacing-md);
		border-radius: var(--radius-sm, 4px);
		background: none;
		border: 1px solid transparent;
		color: var(--color-text-secondary);
	}

	.tabs button.active {
		border-color: var(--color-border-subtle);
		background-color: var(--color-bg-elevated);
		color: var(--color-text-primary, inherit);
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

	.overviewStage {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-self: center;
		width: min(640px, 100%);
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
		overflow-y: auto;
		min-height: 0;
	}

	.castStage {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-self: center;
		width: min(720px, 100%);
		gap: var(--spacing-lg);
		padding: var(--spacing-lg);
		overflow-y: auto;
		min-height: 0;
	}

	.hint {
		color: var(--color-text-muted);
		line-height: 1.5;
		font-size: 0.9rem;
	}

	.hint.centered {
		margin: auto;
		max-width: 480px;
		text-align: center;
	}
</style>
