<script lang="ts">
	import type { StudioSection } from '$lib/navigation/routes.js';
	import type { Snowflake } from '$lib/types/core.js';
	import StudioNav from '../studio/StudioNav.svelte';
	import LorebooksSection from '../studio/sections/lorebook/LorebooksSection.svelte';
	import TemplateSection from '../studio/sections/templates/TemplateSection.svelte';
	import CharacterSection from '../studio/sections/characters/CharacterSection.svelte';
	import RealmSection from '../studio/sections/realm/RealmSection.svelte';
	import PersonaSection from '../studio/sections/personas/PersonaSection.svelte';
	import ToolsSection from '../studio/sections/tools/ToolsSection.svelte';
	import WidgetsSection from '../studio/sections/widgets/WidgetsSection.svelte';

	let { section, id }: { section: StudioSection; id?: Snowflake } = $props();
</script>

<div style="display:flex; flex-direction:column; flex:1; min-height: 0;">
	<StudioNav {section} />

	{#if section === 'lorebooks'}
		<LorebooksSection {id} />
	{:else if section === 'templates'}
		<TemplateSection {id} />
	{:else if section === 'characters'}
		<CharacterSection {id} />
	{:else if section === 'tools'}
		<ToolsSection {id} />
	{:else if section === 'personas'}
		<PersonaSection {id} />
	{:else if section === 'widgets'}
		<WidgetsSection {id} />
	{:else if section === 'realm'}
		<RealmSection />
	{/if}
</div>
