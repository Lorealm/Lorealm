<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import TemplateList from './TemplateList.svelte';
	import TemplateEditor from './TemplateEditor.svelte';
	import WideScreenNote from '$lib/components/WideScreenNote.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	<WideScreenNote subject="The template editor" />
	{#key id}
		<TemplateEditor />
	{/key}
{:else}
	<TemplateList />
{/if}
