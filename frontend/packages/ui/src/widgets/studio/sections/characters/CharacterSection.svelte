<script lang="ts">
	import { studioStore } from '$lib/stores/studio.svelte';
	import CharacterList from './CharacterList.svelte';
	import CharacterEditor from './CharacterEditor.svelte';
</script>

{#if studioStore.editingId}
	<CharacterEditor id={studioStore.editingId} />
{:else}
	<CharacterList />
{/if}
