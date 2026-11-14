<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import TemplateList from './TemplateList.svelte';
	import TemplateEditor from './TemplateEditor.svelte';
	import WideScreenNote from '#lib/components/WideScreenNote.svelte';

	let { id }: { id?: Snowflake } = $props();
</script>

{#if id}
	<WideScreenNote subject={t.studio.sections.templateEditor} />
	{#key id}
		<TemplateEditor />
	{/key}
{:else}
	<TemplateList />
{/if}
