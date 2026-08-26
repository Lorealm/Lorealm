<script lang="ts">
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { ArrowUpDown, Plus, Upload } from '@lucide/svelte';
	import CharacterCard from './CharacterCard.svelte';
	import { showToast } from '$lib/stores/errorToast.svelte.js';
	import type { ImportedLorebook, ImportResult } from '$lib/types/character.js';

	type SortKey = 'name-asc' | 'name-desc' | 'updated' | 'created';

	let {
		oncreate
	}: {
		oncreate?: () => void;
	} = $props();

	type SortableMeta = {
		id: string;
		name?: string;
		createdAt?: string | number | Date;
		updatedAt?: string | number | Date;
	};

	let sortBy = $state<SortKey>('name-asc');

	const characters = $derived(characterStore.metadataList);

	let inputEl: HTMLInputElement;
	let busy = $state(false);

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
			results = await characterStore.import(files, true);
		} finally {
			busy = false;
		}

		const lorebooks = results.flatMap((r) => (r.status === 'ok' && r.lorebook ? [r.lorebook] : []));
		reportImportedLorebooks(lorebooks);
	}

	function reportImportedLorebooks(lorebooks: ImportedLorebook[]) {
		if (lorebooks.length === 0) return;

		const entries = lorebooks.reduce((sum, b) => sum + b.entry_count, 0);
		const skipped = lorebooks.reduce((sum, b) => sum + b.skipped, 0);

		const subject =
			lorebooks.length === 1 ? `“${lorebooks[0].name}”` : `${lorebooks.length} lorebooks`;
		const entryCount = `${entries} ${entries === 1 ? 'entry' : 'entries'}`;
		const tail = skipped > 0 ? ` ${skipped} could not be converted.` : '';

		showToast({
			title: 'Lorebook imported',
			message: `Imported ${subject} with ${entryCount}.${tail}`,
			variant: skipped > 0 ? 'warning' : 'success'
		});
	}
</script>

<input
	bind:this={inputEl}
	type="file"
	accept="image/*"
	onchange={onPick}
	hidden
	style="display:none"
	multiple
/>

<div class="page">
	<div class="topbar">
		<div class="sort">
			<ArrowUpDown size={16} />
			<select bind:value={sortBy} aria-label="Sort characters">
				<option value="name-asc">Name (A–Z)</option>
				<option value="name-desc">Name (Z–A)</option>
				<option value="updated">Recently updated</option>
				<option value="created">Recently added</option>
			</select>
		</div>

		<div class="actions">
			<button type="button" onclick={openPicker} disabled={busy}>
				<Upload size={16} />
				{busy ? 'Uploading…' : 'Import'}
			</button>
			<button type="button" class="primary" onclick={() => oncreate?.()}>
				<Plus size={16} />
				New
			</button>
		</div>
	</div>

	{#if sortedCharacters.length === 0}
		<div class="empty">No characters yet — create one to get started.</div>
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
		backdrop-filter: var(--backdrop-blur);
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
</style>
