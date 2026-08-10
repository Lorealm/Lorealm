<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import LorebookList from './LorebookList.svelte';
	import LorebookEditor from './LorebookEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	{#key id}
		<LorebookEditor />
	{/key}
{:else}
	<LorebookList />
{/if}
