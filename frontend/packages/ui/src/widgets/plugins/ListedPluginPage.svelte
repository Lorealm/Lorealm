<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { Check, Download } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import { marketplaceStore, shownVersion } from '#lib/stores/marketplace.svelte.js';
	import { pluginsStore } from '#lib/stores/plugins.svelte.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import { canInstall, change, installListed } from './actions.svelte.js';
	import PluginDetail from './PluginDetail.svelte';

	let { handle }: { handle: string } = $props();

	const api = useApi();

	const listed = $derived(marketplaceStore.find(handle));
	const version = $derived(listed && shownVersion(listed));
	const mine = $derived(pluginsStore.plugins.find((p) => p.handle === handle));
	const update = $derived(canInstall() ? marketplaceStore.updateFor(handle) : null);

	let error = $state('');
	let busy = $state(false);

	onMount(() => {
		void marketplaceStore.load();
		void pluginsStore.load();
		void configStore.loadPermissions();
	});

	async function act(action: () => Promise<unknown>) {
		busy = true;
		error = await change(action);
		busy = false;
	}

	function readme(): Promise<string> {
		if (!listed?.has_readme) return Promise.reject();
		return api.plugins.listedReadme(handle);
	}
</script>

{#if !marketplaceStore.loaded}
	<div class="empty">{t.plugins.loading}</div>
{:else if !listed || !version}
	<div class="empty">{t.plugins.notListed}</div>
{:else}
	<PluginDetail plugin={version} {readme} repository={listed.repository} {error}>
		{#snippet actions()}
			{#if mine}
				<button
					type="button"
					class:on={mine.enabled}
					disabled={busy}
					onclick={() => act(() => api.plugins.toggle(handle, !mine.enabled))}
				>
					{#if mine.enabled}<Check size={14} /> {t.plugins.enabled}{:else}{t.plugins.enable}{/if}
				</button>
				{#if update}
					<button type="button" disabled={busy} onclick={() => act(() => installListed(handle))}>
						{t.plugins.updateTo(update)}
					</button>
				{/if}
				<button type="button" onclick={() => navigator.start().plugins(handle)}
					>{t.plugins.manage}</button
				>
			{:else if listed.offered && canInstall()}
				<button
					type="button"
					class="primary"
					disabled={busy}
					onclick={() => act(() => installListed(handle))}
				>
					<Download size={14} />
					{busy ? t.plugins.installing : t.plugins.install}
				</button>
			{/if}
		{/snippet}
		{#snippet notice()}
			{#if !listed.offered}
				<p class="muted">{t.plugins.noCompatible}</p>
			{:else if !mine && !canInstall()}
				<p class="muted">{t.plugins.notOnServer}</p>
			{/if}
		{/snippet}
	</PluginDetail>
{/if}

<style>
	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
	}
</style>
