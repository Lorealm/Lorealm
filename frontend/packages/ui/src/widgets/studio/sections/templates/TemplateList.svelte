<script lang="ts">
	import { templateStore } from '$lib/stores/template.svelte.js';
	import { onMount } from 'svelte';
	import TemplateCard from './TemplateCard.svelte';

	const templates = $derived(templateStore.metadataList);

	onMount(async () => {
		await templateStore.loadList();
		await templateStore.loadActive();
	});
</script>

{#each templates as template (template.id)}
	<TemplateCard id={template.id} />
{/each}
