<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onDestroy } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import EntryList from './entries/EntryList.svelte';
	import EntryEditor from './entries/EntryEditor.svelte';
	import LorebookConfigEditor from './config/LorebookConfigEditor.svelte';
	import SaveIndicator from '#lib/components/SaveIndicator.svelte';
	import { SvelteMap } from 'svelte/reactivity';

	const SAVE_MS = 500;

	const draft = $derived(lorebookStore.editingLorebook);

	let selected = $state<Snowflake | 'config'>('config');
	/** A narrow window shows the entry list or what it opened, never both. */
	let paneOpen = $state(false);
	const selectedEntry = $derived(
		selected !== 'config' && draft
			? (draft.entries.find((e) => e.snowflake === selected) ?? null)
			: null
	);

	$effect(() => {
		const pending = lorebookStore.pendingEntry;
		if (pending && draft?.entries.some((e) => e.snowflake === pending)) {
			selected = pending;
			lorebookStore.pendingEntry = null;
		}
	});

	const entryTimers = new SvelteMap<Snowflake, ReturnType<typeof setTimeout>>();
	let bookTimer: ReturnType<typeof setTimeout> | undefined;

	function scheduleEntrySave(eid: Snowflake) {
		clearTimeout(entryTimers.get(eid));
		entryTimers.set(
			eid,
			setTimeout(() => void lorebookStore.saveEntry(eid), SAVE_MS)
		);
	}
	function scheduleBookSave() {
		clearTimeout(bookTimer);
		bookTimer = setTimeout(() => void lorebookStore.saveLorebook(), SAVE_MS);
	}
	onDestroy(() => {
		for (const t of entryTimers.values()) clearTimeout(t);
		clearTimeout(bookTimer);
	});

	async function addEntry() {
		const eid = await lorebookStore.addEntry();
		if (eid) {
			selected = eid;
			paneOpen = true;
		}
	}
	function toggleEntry(eid: Snowflake) {
		const entry = draft?.entries.find((e) => e.snowflake === eid);
		if (!entry) return;
		entry.meta.enabled = !(entry.meta.enabled ?? true);
		void lorebookStore.saveEntry(eid); // save immediately. no debounce
	}
	async function deleteEntry(eid: Snowflake) {
		if (selected === eid) selected = 'config';
		try {
			await lorebookStore.deleteEntry(eid);
		} catch (e) {
			console.error(e);
		}
	}
</script>

<div class="lorebook-editor" class:pane-open={paneOpen}>
	<section class="col-list">
		<header class="list-header">
			<div class="header-row">
				<button
					class="back"
					onclick={() => navigator.start().studio().lorebooks()}
					aria-label={t.studio.lorebookEditor.back}>←</button
				>
				<span class="book-name" {@attach tooltip(draft?.name)}
					>{draft?.name ?? t.studio.lorebookEditor.loading}</span
				>
			</div>
			<div class="header-row">
				<button
					class="settings"
					class:active={selected === 'config'}
					type="button"
					onclick={() => {
						selected = 'config';
						paneOpen = true;
					}}>{t.studio.lorebookEditor.settings}</button
				>
				<button class="add" type="button" onclick={addEntry}
					>{t.studio.lorebookEditor.addEntry}</button
				>
			</div>
			<SaveIndicator state={lorebookStore.bookSaveState} />
		</header>

		{#if draft}
			<EntryList
				entries={draft.entries}
				selectedId={selected === 'config' ? null : selected}
				defaultPriority={draft.config.default_priority}
				onSelect={(eid) => {
					selected = eid;
					paneOpen = true;
				}}
				onToggle={toggleEntry}
				onDelete={deleteEntry}
				entryState={(eid) => lorebookStore.entrySaveState(eid)}
			/>
		{/if}
	</section>

	<section class="col-content">
		<button type="button" class="to-list" onclick={() => (paneOpen = false)}
			>{t.studio.lorebookEditor.entries}</button
		>
		{#if selectedEntry && draft}
			{#key selectedEntry.snowflake}
				<EntryEditor
					entry={selectedEntry}
					config={draft.config}
					saveState={lorebookStore.entrySaveState(selectedEntry.snowflake)}
					requestSave={() => scheduleEntrySave(selectedEntry.snowflake)}
				/>
			{/key}
		{:else if draft}
			<LorebookConfigEditor
				{draft}
				saveState={lorebookStore.bookSaveState}
				requestSave={scheduleBookSave}
			/>
		{/if}
	</section>
</div>

<style>
	.lorebook-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
	}
	.col-list {
		flex: 0 0 300px;
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-right: 1px solid var(--color-border-subtle);
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
	}
	.list-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.header-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.back {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.book-name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.settings,
	.add {
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.settings.active {
		font-weight: bold;
		color: var(--color-text-primary);
	}
	.to-list {
		display: none;
	}
	@media (max-width: 768px) {
		.col-list {
			flex: 1 1 auto;
			border-right: none;
		}
		.col-content {
			flex-direction: column;
		}
		.lorebook-editor.pane-open .col-list,
		.lorebook-editor:not(.pane-open) .col-content {
			display: none;
		}
		.to-list {
			display: flex;
			flex-shrink: 0;
			align-self: flex-start;
			margin: var(--spacing-sm) var(--spacing-md) 0;
			font-size: var(--font-size-sm);
			padding: var(--spacing-xs) var(--spacing-sm);
		}
	}
</style>
