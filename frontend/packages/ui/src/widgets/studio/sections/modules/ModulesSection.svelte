<script lang="ts">
	import type { Snowflake } from '#lib/types/core.js';
	import ModulesEditor from './ModulesEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

<ModulesEditor selectedId={id ?? null} />
