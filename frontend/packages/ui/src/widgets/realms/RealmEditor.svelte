<script lang="ts">
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import AccessMapTab from './AccessMapTab.svelte';
	import CastTab from './CastTab.svelte';
	import FormattingTab from './FormattingTab.svelte';
	import GraphTab from './GraphTab.svelte';
	import HistoryTab from './HistoryTab.svelte';
	import OverviewTab from './OverviewTab.svelte';
	import ScenariosTab from './ScenariosTab.svelte';
	import { EDITOR_TABS, realmEditor, type EditorTab } from './realmEditor.svelte.js';

	async function select(tab: EditorTab) {
		// Persist any pending Cast edits before the tab unmounts.
		await realmStore.flushAgentSaves();
		realmEditor.setTab(tab);
	}
</script>

<div class="editor">
	<nav class="tabs">
		{#each EDITOR_TABS as tab (tab.value)}
			<button
				type="button"
				class:active={realmEditor.tab === tab.value}
				onclick={() => select(tab.value)}
			>
				{tab.label}
			</button>
		{/each}
	</nav>

	<div class="content">
		{#if realmEditor.tab === 'overview'}
			<OverviewTab />
		{:else if realmEditor.tab === 'cast'}
			<CastTab />
		{:else if realmEditor.tab === 'scenarios'}
			<ScenariosTab />
		{:else if realmEditor.tab === 'access'}
			<AccessMapTab />
		{:else if realmEditor.tab === 'graph'}
			<GraphTab />
		{:else if realmEditor.tab === 'formatting'}
			<FormattingTab />
		{:else if realmEditor.tab === 'history'}
			<HistoryTab />
		{/if}
	</div>
</div>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.tabs {
		display: flex;
		gap: 2px;
		flex-shrink: 0;
		padding: var(--spacing-lg) var(--spacing-2xl) 0;
		border-bottom: 1px solid var(--color-border-subtle);
	}

	.tabs button {
		padding: var(--spacing-sm) var(--spacing-md);
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-text-secondary);
		font-size: var(--font-size-base);
		white-space: nowrap;
	}

	.tabs button:hover {
		color: var(--color-text-primary);
	}

	.tabs button.active {
		color: var(--color-text-primary);
		border-bottom-color: var(--color-primary);
	}

	.content {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-2xl);
	}
</style>
