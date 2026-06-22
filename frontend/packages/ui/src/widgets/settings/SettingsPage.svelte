<script lang="ts">
	import { settingsNav, SettingsScope } from '$lib/stores/settings.svelte';
	import { configStore } from '$lib/stores/serverConfig.svelte';
	import UserSettings from './UserSettings.svelte';
	import ServerSettings from './ServerSettings.svelte';
</script>

<div class="settings">
	<nav class="scopeNav">
		<button
			class="tab"
			class:active={settingsNav.scope === SettingsScope.User}
			onclick={() => settingsNav.setScope(SettingsScope.User)}
		>
			User
		</button>
		{#if configStore.canEditServer}
			<button
				class="tab"
				class:active={settingsNav.scope === SettingsScope.Server}
				onclick={() => settingsNav.setScope(SettingsScope.Server)}
			>
				Server
			</button>
		{/if}
	</nav>

	{#if settingsNav.scope === SettingsScope.Server && configStore.canEditServer}
		<ServerSettings />
	{:else}
		<UserSettings />
	{/if}
</div>

<style>
	.settings {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		padding: var(--spacing-sm);
	}
	.scopeNav {
		display: flex;
		gap: var(--spacing-sm);
		flex-shrink: 0;
		padding-bottom: var(--spacing-sm);
		margin-bottom: var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
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
