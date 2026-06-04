<script lang="ts">
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import { onMount } from 'svelte';
	import LorebookCard from './LorebookCard.svelte';

	const lorebooks = $derived(lorebookStore.metadataList);

	onMount(async () => {
		await lorebookStore.loadList();
		await lorebookStore.loadActive();
	});
</script>

{#each lorebooks as book (book.id)}
	<LorebookCard id={book.id} />
{/each}
