<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';

	// placeholder — replace with a load from CharacterService-style storage
	const lorebooks = [
		{ id: 'codex', name: 'Eldoria Codex' },
		{ id: 'north', name: 'Northern Reaches' }
	];
</script>

{#each lorebooks as book (book.id)}
	<button onclick={() => studioStore.open(book.id)}>{book.name}</button>
{/each}
