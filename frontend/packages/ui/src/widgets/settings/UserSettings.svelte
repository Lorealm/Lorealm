<script lang="ts">
	import { navigator } from '$lib/navigation/navigator.js';
	import type { UserSection } from '$lib/navigation/routes.js';
	import TokenizersSection from './user/TokenizersSection.svelte';

	let { section }: { section: UserSection } = $props();

	// Inline nav while there's one section. Extract a UserSettingsNav (mirroring
	// ServerSettingsNav) once more sections land.
	const sections: Array<{ id: UserSection; label: string; go: () => void }> = [
		{
			id: 'tokenizers',
			label: 'Tokenizers',
			go: () => navigator.start().settings().user().tokenizers()
		}
	];
</script>

<div class="user">
	<nav class="nav">
		{#each sections as { id, label, go } (id)}
			<button class="tab" class:active={section === id} onclick={go}>
				{label}
			</button>
		{/each}
	</nav>

	{#if section === 'tokenizers'}
		<TokenizersSection />
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
</style>
