<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import TemplateList from './TemplateList.svelte';
	import TemplateEditor from './TemplateEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	{#key id}
		<TemplateEditor />
	{/key}
{:else}
	<TemplateList />
{/if}
