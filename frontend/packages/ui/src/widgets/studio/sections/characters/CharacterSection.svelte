<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';
	import CharacterList from './CharacterList.svelte';
	import CharacterEditor from './CharacterEditor.svelte';
	import { characterStore } from '$lib/stores/character.svelte.js';

	async function handleCreate() {
		const id = await characterStore.create('New Character');
		studioStore.open(id);
	}
</script>

{#if studioStore.editingId}
	<CharacterEditor id={studioStore.editingId} />
{:else}
	<CharacterList oncreate={handleCreate} />
{/if}
