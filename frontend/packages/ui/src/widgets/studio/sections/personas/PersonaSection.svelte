<script lang="ts">
	import type { Snowflake } from '#lib/types/core.js';
	import PersonaEditor from './PersonaEditor.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

<PersonaEditor selectedId={id ?? null} />
