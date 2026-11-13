<script lang="ts">
	import { navigator } from '#lib/navigation/navigator.js';
	import InstalledPlugins from './InstalledPlugins.svelte';
	import MarketplacePlugins from './MarketplacePlugins.svelte';

	let { tab }: { tab: 'installed' | 'marketplace' } = $props();
</script>

<div class="page">
	<div class="tabs" role="tablist" aria-label="Plugins">
		<button
			type="button"
			role="tab"
			aria-selected={tab === 'installed'}
			class:active={tab === 'installed'}
			onclick={() => navigator.start().plugins()}
		>
			Installed
		</button>
		<button
			type="button"
			role="tab"
			aria-selected={tab === 'marketplace'}
			class:active={tab === 'marketplace'}
			onclick={() => navigator.start().marketplace()}
		>
			Marketplace
		</button>
	</div>

	{#if tab === 'marketplace'}
		<MarketplacePlugins />
	{:else}
		<InstalledPlugins />
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.tabs {
		display: flex;
		flex: 0 0 auto;
		gap: var(--spacing-xs);
		padding: 0 var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.tabs button {
		padding: var(--spacing-sm) var(--spacing-md);
		background: transparent;
		border: none;
		border-bottom: 2px solid transparent;
		color: var(--color-text-muted);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h6);
	}
	.tabs button:hover {
		color: var(--color-text-secondary);
	}
	.tabs button.active {
		border-bottom-color: var(--color-primary-light);
		color: var(--color-text-primary);
	}
</style>
