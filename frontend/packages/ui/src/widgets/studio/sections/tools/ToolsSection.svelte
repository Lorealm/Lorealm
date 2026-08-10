<script lang="ts">
	import type { Snowflake } from '$lib/types/core.js';
	import ToolsEditor from './ToolsEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

<ToolsEditor selectedId={id ?? null} />
