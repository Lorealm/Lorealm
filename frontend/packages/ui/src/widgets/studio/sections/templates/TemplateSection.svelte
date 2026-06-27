<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';
	import TemplateList from './TemplateList.svelte';
	import TemplateEditor from './TemplateEditor.svelte';
</script>

{#if studioStore.editingId}
	<TemplateEditor id={studioStore.editingId} />
{:else}
	<TemplateList />
{/if}
