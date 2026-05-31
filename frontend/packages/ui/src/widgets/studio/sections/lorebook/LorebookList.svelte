<script lang="ts">
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte';
	import { onMount } from 'svelte';

	const lorebooks = $derived(lorebookStore.metadataList);

	onMount(async () => {
		await lorebookStore.loadList();
	});
</script>

{#each lorebooks as book (book.id)}
	<button onclick={() => studioStore.open(book.id)}>{book.name}</button>
{/each}
