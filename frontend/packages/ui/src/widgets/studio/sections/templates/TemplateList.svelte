<script lang="ts">
	import { templateStore } from '$lib/stores/template.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import { onMount } from 'svelte';
	import TemplateCard from './TemplateCard.svelte';
	import StudioList from '../StudioList.svelte';

	let query = $state('');
	let sort = $state('updated');
	let creating = $state(false);

	const templates = $derived(templateStore.metadataList ?? []);

	const ts = (value: string): number => {
		const t = new Date(value).getTime();
		return Number.isNaN(t) ? 0 : t;
	};

	const shown = $derived.by(() => {
		const needle = query.trim().toLowerCase();
		const list = templates.filter(
			(t) =>
				needle === '' ||
				t.name.toLowerCase().includes(needle) ||
				t.description.toLowerCase().includes(needle)
		);
		if (sort === 'name-asc') return list.sort((a, b) => a.name.localeCompare(b.name));
		if (sort === 'name-desc') return list.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'nodes') return list.sort((a, b) => b.node_count - a.node_count);
		return list.sort((a, b) => ts(b.updated_at) - ts(a.updated_at));
	});

	onMount(async () => {
		await templateStore.loadList();
		await templateStore.loadActive();
	});

	async function create() {
		if (creating) return;
		creating = true;
		try {
			const tpl = await templateStore.createTemplate({
				name: 'Untitled template',
				default: true
			});
			studioStore.open(tpl.id);
		} catch (e) {
			console.error(e);
		} finally {
			creating = false;
		}
	}
</script>

<StudioList
	title="Templates"
	bind:query
	bind:sort
	sortOptions={[
		{ value: 'updated', label: 'Recently updated' },
		{ value: 'name-asc', label: 'Name (A–Z)' },
		{ value: 'name-desc', label: 'Name (Z–A)' },
		{ value: 'nodes', label: 'Most nodes' }
	]}
	searchPlaceholder="Search templates…"
	createLabel="New template"
	oncreate={create}
	{creating}
>
	{#snippet empty()}
		{#if shown.length === 0}
			<p class="empty">
				{templates.length === 0
					? 'No templates yet — create one to shape how context reaches the model.'
					: 'No templates match that search.'}
			</p>
		{/if}
	{/snippet}

	{#each shown as template (template.id)}
		<TemplateCard meta={template} />
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
