<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';
	import LorebookList from './LorebookList.svelte';
	import LorebookEditor from './LorebookEditor.svelte';
</script>

{#if studioStore.editingId}
	<LorebookEditor id={studioStore.editingId} />
{:else}
	<LorebookList />
{/if}
