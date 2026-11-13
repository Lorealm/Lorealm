<script lang="ts">
	import { navigator } from '#lib/navigation/navigator.js';
	import type { UserSection } from '#lib/navigation/routes.js';
	import ProfileSection from './user/ProfileSection.svelte';
	import TokenizersSection from './user/TokenizersSection.svelte';
	import PluginsSection from './user/PluginsSection.svelte';
	import AppearanceSection from './client/AppearanceSection.svelte';
	import EditorSection from './client/EditorSection.svelte';
	import InspectorSection from './client/InspectorSection.svelte';
	import KeybindsSection from './client/KeybindsSection.svelte';

	let { section, id }: { section: UserSection; id?: string } = $props();

	type Tab = { id: UserSection; label: string; go: () => void };

	const account: Tab[] = [
		{ id: 'profile', label: 'Profile', go: () => navigator.start().settings().user().profile() },
		{
			id: 'tokenizers',
			label: 'Tokenizers',
			go: () => navigator.start().settings().user().tokenizers()
		},
		{ id: 'plugins', label: 'Plugins', go: () => navigator.start().settings().user().plugins() }
	];

	const device: Tab[] = [
		{
			id: 'appearance',
			label: 'Appearance',
			go: () => navigator.start().settings().user().appearance()
		},
		{ id: 'editor', label: 'Editor', go: () => navigator.start().settings().user().editor() },
		{
			id: 'inspector',
			label: 'Inspector',
			go: () => navigator.start().settings().user().inspector()
		},
		{ id: 'keybinds', label: 'Keybinds', go: () => navigator.start().settings().user().keybinds() }
	];
</script>

<div class="user">
	<nav class="nav">
		{#each account as { id, label, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>{label}</button>
		{/each}

		<span class="divider" role="separator"></span>

		{#each device as { id, label, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>{label}</button>
		{/each}
	</nav>

	{#if section === 'profile'}
		<ProfileSection />
	{:else if section === 'tokenizers'}
		<TokenizersSection />
	{:else if section === 'plugins'}
		<PluginsSection handle={id} />
	{:else if section === 'appearance'}
		<AppearanceSection />
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
