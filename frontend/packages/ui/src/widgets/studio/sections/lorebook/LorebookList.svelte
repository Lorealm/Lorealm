<script lang="ts">
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import LorebookCard from './LorebookCard.svelte';
	import StudioList from '../StudioList.svelte';

	let query = $state('');
	let sort = $state('name-asc');
	let creating = $state(false);

	const lorebooks = $derived(lorebookStore.metadataList ?? []);

	const shown = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const list = lorebooks.filter(
			(b) =>
				needle === '' ||
				b.name.toLowerCase().includes(needle) ||
				b.description.toLowerCase().includes(needle)
		);
		if (sort === 'name-desc') return list.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'entries') return list.sort((a, b) => b.entry_count - a.entry_count);
		if (sort === 'active')
			return list.sort(
				(a, b) => Number(lorebookStore.isActive(b.id)) - Number(lorebookStore.isActive(a.id))
			);
		return list.sort((a, b) => a.name.localeCompare(b.name));
	});

	async function create() {
		if (creating) return;
		creating = true;
		try {
			const book = await lorebookStore.createLorebook({ name: 'Untitled lorebook' });
			navigator.start().studio().lorebooks(book.id);
		} catch (e) {
			console.error(e);
		} finally {
			creating = false;
		}
	}
</script>

<StudioList
	title="Lorebooks"
	bind:query
	bind:sort
	sortOptions={[
		{ value: 'name-asc', label: 'Name (A–Z)' },
		{ value: 'name-desc', label: 'Name (Z–A)' },
		{ value: 'entries', label: 'Most entries' },
		{ value: 'active', label: 'Active first' }
	]}
	searchPlaceholder="Search lorebooks…"
	createLabel="New lorebook"
	oncreate={create}
	{creating}
>
	{#snippet empty()}
		{#if shown.length === 0}
			<p class="empty">
				{lorebooks.length === 0
					? 'No lorebooks yet — create one to start writing world knowledge.'
					: 'No lorebooks match that search.'}
			</p>
		{/if}
	{/snippet}

	{#each shown as book (book.id)}
		<LorebookCard meta={book} />
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
