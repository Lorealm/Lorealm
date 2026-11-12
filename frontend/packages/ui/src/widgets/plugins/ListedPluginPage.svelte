<script lang="ts">
	import { onMount } from 'svelte';
	import { Check, Download } from '@lucide/svelte';
	import { useApi } from '$lib/api/client.js';
	import { navigator } from '$lib/navigation/navigator.js';
	import { marketplaceStore, shownVersion } from '$lib/stores/marketplace.svelte.js';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import { configStore } from '$lib/stores/serverConfig.svelte.js';
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
	<div class="empty">Loading…</div>
{:else if !listed || !version}
	<div class="empty">This plugin isn't in the marketplace.</div>
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
					{#if mine.enabled}<Check size={14} /> Enabled{:else}Enable{/if}
				</button>
				{#if update}
					<button type="button" disabled={busy} onclick={() => act(() => installListed(handle))}>
						Update to {update}
					</button>
				{/if}
				<button type="button" onclick={() => navigator.start().plugins(handle)}>Manage</button>
			{:else if listed.offered && canInstall()}
				<button
					type="button"
					class="primary"
					disabled={busy}
					onclick={() => act(() => installListed(handle))}
				>
					<Download size={14} />
					{busy ? 'Installing…' : 'Install'}
				</button>
			{/if}
		{/snippet}
		{#snippet notice()}
			{#if !listed.offered}
				<p class="muted">No version of it works with this version of Lorealm.</p>
			{:else if !mine && !canInstall()}
				<p class="muted">
					It isn't on this server yet. Someone who manages its plugins can add it.
				</p>
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
