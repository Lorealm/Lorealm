<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import { ArrowUpDown, Plus, Upload } from '@lucide/svelte';
	import CharacterCard from './CharacterCard.svelte';
	import { showToast } from '#lib/stores/errorToast.svelte.js';
	import { memory, type CharacterSort } from '#lib/stores/memory.svelte.js';
	import type { AttachmentReport, ImportedLorebook, ImportResult } from '#lib/types/character.js';
	import { tooltip } from '#lib/stores/tooltip.svelte.js';
	import { extractError } from '#lib/types/error.js';
	import { navigator } from '#lib/navigation/navigator.js';

	type SortableMeta = {
		id: string;
		name?: string;
		createdAt?: string | number | Date;
		updatedAt?: string | number | Date;
	};

	const sortBy = $derived(memory.current.charactersSort);

	const characters = $derived(characterStore.metadataList);

	let inputEl: HTMLInputElement;
	let busy = $state(false);
	let reuseLorebooks = $state(false);

	const ts = (v: unknown): number => {
		if (!v) return 0;
		const t = new Date(v as string | number | Date).getTime();
		return Number.isNaN(t) ? 0 : t;
	};

	const sortedCharacters = $derived.by(() => {
		const list = [...characters] as unknown as SortableMeta[];
		switch (sortBy) {
			case 'name-asc':
				list.sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''));
				break;
			case 'name-desc':
				list.sort((a, b) => (b.name ?? '').localeCompare(a.name ?? ''));
				break;
			case 'updated':
				list.sort((a, b) => ts(b.updatedAt) - ts(a.updatedAt));
				break;
			case 'created':
				list.sort((a, b) => ts(b.createdAt) - ts(a.createdAt));
				break;
		}
		return list as unknown as typeof characters;
	});

	function openPicker() {
		inputEl.click();
	}

	async function onPick(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const files = Array.from(input.files ?? []);
		input.value = '';
		if (files.length === 0) return;

		let results: ImportResult[];
		busy = true;
		try {
			results = await characterStore.import(files, true, { reuseLorebooks });
		} finally {
			busy = false;
		}

		const lorebooks = results.flatMap((r) => (r.status === 'ok' && r.lorebook ? [r.lorebook] : []));
		reportImportedLorebooks(lorebooks);
		reportAttachments(
			results.flatMap((r) => (r.status === 'ok' && r.attachments ? [r.attachments] : []))
		);
	}

	function reportAttachments(reports: AttachmentReport[]) {
		if (reports.length === 0) return;

		const reused = reports.reduce((sum, r) => sum + r.reused, 0);
		const created = reports.reduce((sum, r) => sum + r.created, 0);
		const renamed = reports.flatMap((r) => r.renamed);
		const skipped = reports.flatMap((r) => r.skipped);

		const parts = [];
		if (created > 0) parts.push(t.characters.attachments.imported({ count: created }));
		if (reused > 0) parts.push(t.characters.attachments.reused({ count: reused }));
		if (renamed.length > 0)
			parts.push(
				t.characters.attachments.renamed({
					list: renamed
						.map(([from, to]) => t.characters.attachments.rename({ from, to }))
						.join(', ')
				})
			);
		if (skipped.length > 0)
			parts.push(t.characters.attachments.skipped({ list: skipped.join(', ') }));

		showToast({
			title: t.characters.attachments.title,
			message: parts.join(' '),
			variant: skipped.length > 0 ? 'warning' : 'success'
		});
	}

	function reportImportedLorebooks(lorebooks: ImportedLorebook[]) {
		if (lorebooks.length === 0) return;

		const entries = lorebooks.reduce((sum, b) => sum + b.entry_count, 0);
		const skipped = lorebooks.reduce((sum, b) => sum + b.skipped, 0);

		const subject =
			lorebooks.length === 1
				? t.characters.lorebookImport.named({ name: lorebooks[0].name })
				: t.characters.lorebookImport.lorebooks({ count: lorebooks.length });
		const entryCount = t.characters.lorebookImport.entries({ count: entries });

		const parts = [t.characters.lorebookImport.message({ subject, entries: entryCount })];
		if (skipped > 0) parts.push(t.characters.lorebookImport.notConverted({ count: skipped }));

		showToast({
			title: t.characters.lorebookImport.title,
			message: parts.join(' '),
			variant: skipped > 0 ? 'warning' : 'success'
		});
	}

	async function createAndNavigateToCharacter() {
		try {
			const id = await characterStore.create(t.characters.newName);
			navigator.start().studio().characters(id);
		} catch (e) {
			const error = extractError(e);
			showToast({
				title: t.characters.createFailed,
				message: error.message,
				variant: 'error'
			});
		}
	}
</script>

<input
	bind:this={inputEl}
	type="file"
	accept="image/png,.png,application/json,.json"
	onchange={onPick}
	hidden
	style="display:none"
	multiple
/>

<div class="page">
	<div class="topbar">
		<div class="sort">
			<ArrowUpDown size={16} />
			<select
				value={sortBy}
				onchange={(e) => memory.set('charactersSort', e.currentTarget.value as CharacterSort)}
				aria-label={t.characters.sort}
			>
				<option value="name-asc">{t.characters.sortOptions.nameAsc}</option>
				<option value="name-desc">{t.characters.sortOptions.nameDesc}</option>
				<option value="updated">{t.characters.sortOptions.updated}</option>
				<option value="created">{t.characters.sortOptions.created}</option>
			</select>
		</div>

		<div class="actions">
			<label class="reuse" {@attach tooltip(t.characters.reuseHint)}>
				<input type="checkbox" bind:checked={reuseLorebooks} />
				{t.characters.reuse}
			</label>
			<button type="button" onclick={openPicker} disabled={busy}>
				<Upload size={16} />
				{busy ? t.characters.uploading : t.characters.import}
			</button>
			<button type="button" class="primary" onclick={createAndNavigateToCharacter}>
				<Plus size={16} />
				{t.characters.new}
			</button>
		</div>
	</div>

	{#if sortedCharacters.length === 0}
		<div class="empty">{t.characters.empty}</div>
	{:else}
		<div class="list">
			{#each sortedCharacters as char (char.id)}
				<CharacterCard summary={char} />
			{/each}
		</div>
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}

	.topbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
		background-color: var(--color-bg-panel);
		-webkit-backdrop-filter: var(--surface-blur);
		backdrop-filter: var(--surface-blur);
		position: sticky;
		top: 0;
		z-index: var(--z-index-sticky);
	}

	.sort {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-secondary);
	}

	/* Undo the global `select { width: 100% }` so it sizes to the bar */
	.topbar select {
		width: auto;
		min-width: 150px;
		flex: 0 0 auto;
	}

	.actions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
	}
	.reuse {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		cursor: pointer;
	}

	.topbar button {
		align-items: center;
		justify-content: center;
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

	.list {
		flex: 1 1 auto;
		min-height: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(var(--grid-card-min), 1fr));
		grid-auto-rows: max-content;
		gap: var(--grid-gap);
		padding: var(--spacing-lg);
		overflow-y: auto;
		align-content: start;
	}

	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
		text-align: center;
	}

	@media (max-width: 768px) {
		.topbar {
			flex-wrap: wrap;
			padding: var(--spacing-md);
		}
		.sort,
		.actions {
			flex: 1 1 100%;
			margin-left: 0;
		}
		.topbar select {
			flex: 1 1 auto;
			min-width: 0;
		}
		.reuse {
			flex: 1 1 auto;
		}
		.list {
			gap: var(--spacing-md);
			padding: var(--spacing-md);
		}
	}
</style>
