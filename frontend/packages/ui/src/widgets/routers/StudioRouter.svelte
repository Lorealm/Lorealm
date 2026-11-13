<script module lang="ts">
	const lorebooksSection = () => import('../studio/sections/lorebook/LorebooksSection.svelte');
	const templateSection = () => import('../studio/sections/templates/TemplateSection.svelte');
	const characterSection = () => import('../studio/sections/characters/CharacterSection.svelte');
	const realmSection = () => import('../studio/sections/realm/RealmSection.svelte');
	const personaSection = () => import('../studio/sections/personas/PersonaSection.svelte');
	const toolsSection = () => import('../studio/sections/tools/ToolsSection.svelte');
	const modulesSection = () => import('../studio/sections/modules/ModulesSection.svelte');
	const widgetsSection = () => import('../studio/sections/widgets/WidgetsSection.svelte');
</script>

<script lang="ts">
	import { lazy } from '#lib/core/lazy.js';
	import type { StudioSection } from '#lib/navigation/routes.js';
	import type { Snowflake } from '#lib/types/core.js';
	import StudioNav from '../studio/StudioNav.svelte';

	let { section, id }: { section: StudioSection; id?: Snowflake } = $props();
</script>

<div style="display:flex; flex-direction:column; flex:1; min-height: 0;">
	<StudioNav {section} />

	{#if section === 'lorebooks'}
		{#await lazy(lorebooksSection) then LorebooksSection}
			<LorebooksSection {id} />
		{/await}
	{:else if section === 'templates'}
		{#await lazy(templateSection) then TemplateSection}
			<TemplateSection {id} />
		{/await}
	{:else if section === 'characters'}
		{#await lazy(characterSection) then CharacterSection}
			<CharacterSection {id} />
		{/await}
	{:else if section === 'tools'}
		{#await lazy(toolsSection) then ToolsSection}
			<ToolsSection {id} />
		{/await}
	{:else if section === 'modules'}
		{#await lazy(modulesSection) then ModulesSection}
			<ModulesSection {id} />
		{/await}
	{:else if section === 'personas'}
		{#await lazy(personaSection) then PersonaSection}
			<PersonaSection {id} />
		{/await}
	{:else if section === 'widgets'}
		{#await lazy(widgetsSection) then WidgetsSection}
			<WidgetsSection {id} />
		{/await}
	{:else if section === 'realm'}
		{#await lazy(realmSection) then RealmSection}
			<RealmSection />
		{/await}
	{/if}
</div>
