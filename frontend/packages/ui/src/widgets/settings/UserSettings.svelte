<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { UserSection } from '#lib/navigation/routes.js';
	import ProfileSection from './user/ProfileSection.svelte';
	import TokenizersSection from './user/TokenizersSection.svelte';
	import PluginsSection from './user/PluginsSection.svelte';
	import InterfaceSection from './client/InterfaceSection.svelte';
	import EditorSection from './client/EditorSection.svelte';
	import InspectorSection from './client/InspectorSection.svelte';
	import KeybindsSection from './client/KeybindsSection.svelte';

	let { section, id }: { section: UserSection; id?: string } = $props();

	type Tab = { id: UserSection; go: () => void };

	const account: Tab[] = [
		{ id: 'profile', go: () => navigator.start().settings().user().profile() },
		{
			id: 'tokenizers',
			go: () => navigator.start().settings().user().tokenizers()
		},
		{ id: 'plugins', go: () => navigator.start().settings().user().plugins() }
	];

	const device: Tab[] = [
		{
			id: 'appearance',
			go: () => navigator.start().settings().user().appearance()
		},
		{ id: 'editor', go: () => navigator.start().settings().user().editor() },
		{
			id: 'inspector',
			go: () => navigator.start().settings().user().inspector()
		},
		{ id: 'keybinds', go: () => navigator.start().settings().user().keybinds() }
	];
</script>

<div class="user">
	<nav class="nav">
		{#each account as { id, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>{t.nav.user[id]}</button>
		{/each}

		<span class="divider" role="separator"></span>

		{#each device as { id, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>{t.nav.user[id]}</button>
		{/each}
	</nav>

	{#if section === 'profile'}
		<ProfileSection />
	{:else if section === 'tokenizers'}
		<TokenizersSection />
	{:else if section === 'plugins'}
		<PluginsSection handle={id} />
	{:else if section === 'appearance'}
		<InterfaceSection />
	{:else if section === 'editor'}
		<EditorSection />
	{:else if section === 'inspector'}
		<InspectorSection />
	{:else if section === 'keybinds'}
		<KeybindsSection />
	{/if}
</div>

<style>
	.user {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}
	.nav {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm) 0;
		flex-shrink: 0;
	}
	.tab {
		padding: 6px 14px;
		border-radius: var(--radius-full);
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
	}
	.tab.active {
		border-color: currentColor;
	}
	.divider {
		width: 1px;
		align-self: stretch;
		margin: 2px var(--spacing-xs);
		background: var(--color-border-medium);
	}
</style>
