<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import WidgetsEditor from './WidgetsEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

<WidgetsEditor selectedId={id ?? null} />
