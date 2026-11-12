<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import WidgetsEditor from './WidgetsEditor.svelte';
	import WideScreenNote from '$lib/components/WideScreenNote.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	<WideScreenNote subject="The widget editor" />
{/if}
<WidgetsEditor selectedId={id ?? null} />
