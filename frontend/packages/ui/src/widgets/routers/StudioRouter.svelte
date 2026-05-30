<script lang="ts">
	import { studioStore, StudioSection } from '$lib/stores/studio.svelte';
	import StudioNav from '../studio/StudioNav.svelte';
	import LorebooksSection from '../studio/sections/lorebook/LorebooksSection.svelte';
	import Placeholder from '../Placeholder.svelte';
	import TemplateSection from '../studio/sections/templates/TemplateSection.svelte';
</script>

<div style="display:flex; flex-direction:column; flex:1; min-height:0; padding:var(--spacing-sm)">
	<StudioNav />

	{#if studioStore.section === StudioSection.Lorebooks}
		<LorebooksSection />
	{:else if studioStore.section === StudioSection.Templates}
		<TemplateSection />
	{:else if studioStore.section === StudioSection.Characters}
		<Placeholder label="Characters" />
	{:else if studioStore.section === StudioSection.Commands}
		<Placeholder label="Commands" />
	{/if}
</div>
