<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import WidgetsEditor from './WidgetsEditor.svelte';
	import WideScreenNote from '#lib/components/WideScreenNote.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	<WideScreenNote subject={t.studio.sections.widgetEditor} />
{/if}
<WidgetsEditor selectedId={id ?? null} />
