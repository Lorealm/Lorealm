<script lang="ts">
	import { characterStore } from '#lib/stores/character.svelte.js';
	import CharacterListItem from './CharacterListItem.svelte';
	import StudioList from '../StudioList.svelte';

	let {
		oncreate
	}: {
		oncreate?: () => void | Promise<void>;
	} = $props();

	let query = $state('');
	let sort = $state('updated');
	let creating = $state(false);

	const characters = $derived(characterStore.metadataList ?? []);

	const ts = (value: string): number => {
		const t = new Date(value).getTime();
		return Number.isNaN(t) ? 0 : t;
	};

	const shown = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const list = characters.filter(
			(c) =>
				needle === '' ||
				c.name.toLowerCase().includes(needle) ||
				(c.display_name ?? '').toLowerCase().includes(needle) ||
				c.tags.some((t) => t.toLowerCase().includes(needle))
		);
		if (sort === 'name-asc') return list.sort((a, b) => a.name.localeCompare(b.name));
		if (sort === 'name-desc') return list.sort((a, b) => b.name.localeCompare(a.name));
		return list.sort((a, b) => ts(b.updated_at) - ts(a.updated_at));
	});

	async function create() {
		if (creating) return;
		creating = true;
		try {
			await oncreate?.();
		} finally {
			creating = false;
		}
	}
</script>

<StudioList
	title="Characters"
	bind:query
	bind:sort
	layout="list"
	sortOptions={[
		{ value: 'updated', label: 'Recently updated' },
		{ value: 'name-asc', label: 'Name (A–Z)' },
		{ value: 'name-desc', label: 'Name (Z–A)' }
	]}
	searchPlaceholder="Search characters…"
	createLabel="New character"
	oncreate={create}
	{creating}
>
	{#snippet empty()}
		{#if shown.length === 0}
			<p class="empty">
				{characters.length === 0
					? 'No characters yet — create one to get started.'
					: 'No characters match that search.'}
			</p>
		{/if}
	{/snippet}

	{#each shown as char (char.id)}
		<CharacterListItem summary={char} />
	{/each}
</StudioList>

<style>
	.empty {
		margin: 0;
		padding: var(--spacing-3xl) 0;
		text-align: center;
		color: var(--color-text-muted);
	}
</style>
