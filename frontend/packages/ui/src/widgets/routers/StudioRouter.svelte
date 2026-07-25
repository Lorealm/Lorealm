<script lang="ts">
	import { studioStore, StudioSection } from '$lib/stores/studio.svelte';
	import StudioNav from '../studio/StudioNav.svelte';
	import LorebooksSection from '../studio/sections/lorebook/LorebooksSection.svelte';
	import TemplateSection from '../studio/sections/templates/TemplateSection.svelte';
	import CharacterSection from '../studio/sections/characters/CharacterSection.svelte';
	import RealmSection from '../studio/sections/realm/RealmSection.svelte';
	import PersonaSection from '../studio/sections/personas/PersonaSection.svelte';
	import ToolsSection from '../studio/sections/tools/ToolsSection.svelte';
	import WidgetsSection from '../studio/sections/widgets/WidgetsSection.svelte';
</script>

<div style="display:flex; flex-direction:column; flex:1; min-height:0; padding:var(--spacing-sm)">
	<StudioNav />

	{#if studioStore.section === StudioSection.Lorebooks}
		<LorebooksSection />
	{:else if studioStore.section === StudioSection.Templates}
		<TemplateSection />
	{:else if studioStore.section === StudioSection.Characters}
		<CharacterSection />
	{:else if studioStore.section === StudioSection.Tools}
		<ToolsSection />
	{:else if studioStore.section === StudioSection.Personas}
		<PersonaSection />
	{:else if studioStore.section === StudioSection.Widgets}
		<WidgetsSection />
	{:else if studioStore.section === StudioSection.Realm}
		<RealmSection />
	{/if}
</div>
