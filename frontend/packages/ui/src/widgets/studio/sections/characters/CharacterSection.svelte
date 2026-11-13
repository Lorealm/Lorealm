<script lang="ts">
	import { navigator } from '#lib/navigation/navigator.js';
	import { characterStore } from '#lib/stores/character.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import CharacterList from './CharacterList.svelte';
	import CharacterEditor from './CharacterEditor.svelte';

	let { id }: { id?: Snowflake } = $props();

	async function handleCreate() {
		const created = await characterStore.create('New Character');
		navigator.start().studio().characters(created);
	}
</script>

{#if id}
	{#key id}
		<CharacterEditor />
	{/key}
{:else}
	<CharacterList oncreate={handleCreate} />
{/if}
