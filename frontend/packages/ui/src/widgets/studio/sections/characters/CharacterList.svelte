<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
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
	title={t.studio.characters.title}
	bind:query
	bind:sort
	layout="list"
	sortOptions={[
		{ value: 'updated', label: t.studio.characters.sortUpdated },
		{ value: 'name-asc', label: t.studio.characters.sortNameAsc },
		{ value: 'name-desc', label: t.studio.characters.sortNameDesc }
	]}
	searchPlaceholder={t.studio.characters.search}
	createLabel={t.studio.characters.new}
	oncreate={create}
	{creating}
>
	{#snippet empty()}
		{#if shown.length === 0}
			<p class="empty">
				{characters.length === 0 ? t.studio.characters.empty : t.studio.characters.noMatches}
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
