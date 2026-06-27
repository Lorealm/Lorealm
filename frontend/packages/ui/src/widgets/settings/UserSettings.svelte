<script lang="ts">
	import { settingsNav, UserSection } from '$lib/stores/settings.svelte';
	import TokenizersSection from './user/TokenizersSection.svelte';

	// Inline nav while there's one section. Extract a UserSettingsNav (mirroring
	// ServerSettingsNav) once more sections land.
	const sections = [{ id: UserSection.Tokenizers, label: 'Tokenizers' }];
</script>

<div class="user">
	<nav class="nav">
		{#each sections as { id, label } (id)}
			<button
				class="tab"
				class:active={settingsNav.userSection === id}
				onclick={() => settingsNav.setUserSection(id)}
			>
				{label}
			</button>
		{/each}
	</nav>

	{#if settingsNav.userSection === UserSection.Tokenizers}
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