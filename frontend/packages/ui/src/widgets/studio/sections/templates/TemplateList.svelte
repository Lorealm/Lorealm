<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { templateStore } from '#lib/stores/template.svelte.js';
	import { navigator } from '#lib/navigation/navigator.js';
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

	async function create() {
		if (creating) return;
		creating = true;
		try {
			const tpl = await templateStore.createTemplate({
				name: t.studio.templates.untitled,
				default: true
			});
			navigator.start().studio().templates(tpl.id);
		} catch (e) {
			console.error(e);
		} finally {
			creating = false;
		}
	}
</script>

<StudioList
	title={t.studio.templates.title}
	bind:query
	bind:sort
	sortOptions={[
		{ value: 'updated', label: t.studio.templates.sortUpdated },
		{ value: 'name-asc', label: t.studio.templates.sortNameAsc },
		{ value: 'name-desc', label: t.studio.templates.sortNameDesc },
		{ value: 'nodes', label: t.studio.templates.sortNodes }
	]}
	searchPlaceholder={t.studio.templates.search}
	createLabel={t.studio.templates.new}
	oncreate={create}
	{creating}
>
	{#snippet empty()}
		{#if shown.length === 0}
			<p class="empty">
				{templates.length === 0 ? t.studio.templates.empty : t.studio.templates.noMatches}
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
