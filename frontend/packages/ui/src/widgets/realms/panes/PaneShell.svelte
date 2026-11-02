<script lang="ts">
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import {
		BookOpen,
		FileCode2,
		Landmark,
		MessageSquare,
		ScrollText,
		Shuffle,
		Users,
		X
	} from '@lucide/svelte';
	import { realmDraft } from '../draft.svelte.js';
	import { paneId, panes, type Pane } from '../panes.svelte.js';
	import CastPane from './CastPane.svelte';
	import DirectorPane from './DirectorPane.svelte';
	import DocumentPane from './DocumentPane.svelte';
	import FormattingPane from './FormattingPane.svelte';
	import ScenariosPane from './ScenariosPane.svelte';
	import OverviewPane from './OverviewPane.svelte';
	import ScriptPane from './ScriptPane.svelte';

	/** Every agent that currently has a script pane's worth of content. */
	const scriptKeys = $derived(
		realmDraft.cast
			.flatMap((entry) => (entry.kind === 'agent' ? [entry.agent] : entry.body))
			.filter((agent) => agent.kind === 'script')
			.map((agent) => agent.key)
	);

	// Editing an agent out of the cast, or away from `script`, closes its pane.
	$effect(() => {
		panes.prune(scriptKeys);
	});

	function label(pane: Pane): string {
		if (pane.kind === 'overview') return 'Overview';
		if (pane.kind === 'scenarios') return 'Scenarios';
		if (pane.kind === 'cast') return 'Cast';
		if (pane.kind === 'document') return 'Document';
		if (pane.kind === 'formatting') return 'Formatting';
		if (pane.kind === 'director') return 'Director';
		return realmDraft.agent(pane.agentKey)?.handle || 'script';
	}

	function iconFor(pane: Pane) {
		if (pane.kind === 'overview') return Landmark;
		if (pane.kind === 'scenarios') return BookOpen;
		if (pane.kind === 'cast') return Users;
		if (pane.kind === 'document') return FileCode2;
		if (pane.kind === 'formatting') return MessageSquare;
		if (pane.kind === 'director') return Shuffle;
		return ScrollText;
	}
</script>

<div class="shell">
	<div class="tabs" role="tablist" aria-label="Open panes">
		{#each panes.open as pane (paneId(pane))}
			{@const id = paneId(pane)}
			{@const active = panes.activeId === id}
			{@const Icon = iconFor(pane)}
			<div class="tab" class:active>
				<button
					type="button"
					role="tab"
					aria-selected={active}
					onclick={() => (panes.activeId = id)}
				>
					<Icon size={12} />
					{label(pane)}
				</button>
				{#if pane.kind === 'script'}
					<button
						type="button"
						class="close"
						{@attach tooltip('Close pane')}
						aria-label="Close {label(pane)}"
						onclick={() => panes.close(id)}
					>
						<X size={11} />
					</button>
				{/if}
			</div>
		{/each}
	</div>

	<div class="body">
		{#if panes.active.kind === 'overview'}
			<OverviewPane />
		{:else if panes.active.kind === 'scenarios'}
			<ScenariosPane />
		{:else if panes.active.kind === 'cast'}
			<CastPane />
		{:else if panes.active.kind === 'document'}
			<DocumentPane />
		{:else if panes.active.kind === 'formatting'}
			<FormattingPane />
		{:else if panes.active.kind === 'director'}
			<DirectorPane />
		{:else}
			{#key panes.active.agentKey}
				<ScriptPane agentKey={panes.active.agentKey} />
			{/key}
		{/if}
	</div>
</div>

<style>
	.shell {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
	.tabs {
		display: flex;
		flex: 0 0 auto;
		align-items: stretch;
		gap: 1px;
		padding: 0 var(--spacing-sm);
		background-color: var(--color-bg-elevated);
		border-bottom: 1px solid var(--color-border-subtle);
		overflow-x: auto;
	}
	.tab {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		border-bottom: 2px solid transparent;
		color: var(--color-text-muted);
	}
	.tab:hover {
		color: var(--color-text-secondary);
	}
	.tab.active {
		border-bottom-color: var(--color-primary-light);
		color: var(--color-text-primary);
	}
	.tab button {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border: none;
		color: inherit;
		font-size: var(--font-size-sm);
		white-space: nowrap;
	}
	.close {
		padding-left: 0 !important;
		opacity: 0;
	}
	.tab:hover .close,
	.close:focus-visible {
		opacity: 1;
	}
	.close:hover {
		color: var(--color-text-accent);
	}
	.body {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		min-height: 0;
	}
</style>
