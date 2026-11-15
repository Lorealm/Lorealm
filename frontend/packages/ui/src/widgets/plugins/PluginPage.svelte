<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { onMount } from 'svelte';
	import { Check, RotateCw, Settings, Trash2 } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import { marketplaceStore } from '#lib/stores/marketplace.svelte.js';
	import { pluginsStore } from '#lib/stores/plugins.svelte.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import { canInstall, change, installListed } from './actions.svelte.js';
	import PluginDetail from './PluginDetail.svelte';
	import PluginLogs from './PluginLogs.svelte';

	let { handle }: { handle: string } = $props();

	const api = useApi();

	const plugin = $derived(pluginsStore.plugins.find((p) => p.handle === handle));
	const local = $derived(
		plugin?.enabled && plugin.latest !== null && plugin.latest !== plugin.version
			? plugin.latest
			: null
	);
	const listed = $derived(canInstall() ? marketplaceStore.updateFor(handle) : null);

	let error = $state('');
	let busy = $state(false);

	onMount(() => {
		void pluginsStore.load();
		void configStore.loadPermissions();
		void marketplaceStore.load();
	});

	async function act(action: () => Promise<unknown>) {
		busy = true;
		error = await change(action);
		busy = false;
	}

	async function uninstall() {
		await act(() => api.plugins.uninstall(handle));
		if (!error) navigator.start().plugins();
	}
</script>

{#if !pluginsStore.loaded}
	<div class="empty">{t.plugins.loading}</div>
{:else if !plugin}
	<div class="empty">{t.plugins.notInstalled}</div>
{:else}
	<PluginDetail
		{plugin}
		readme={() => api.plugins.readme(handle)}
		repository={marketplaceStore.find(handle)?.repository}
		{error}
	>
		{#snippet actions()}
			<button
				type="button"
				class:on={plugin.enabled}
				disabled={busy}
				onclick={() => act(() => api.plugins.toggle(handle, !plugin.enabled))}
			>
				{#if plugin.enabled}<Check size={14} /> {t.plugins.enabled}{:else}{t.plugins.enable}{/if}
			</button>
			{#if plugin.has_settings}
				<button
					type="button"
					aria-label={t.plugins.settingsNamed({ name: plugin.name })}
					{@attach tooltip(t.plugins.settings)}
					onclick={() => navigator.start().settings().user().plugins(handle)}
				>
					<Settings size={14} />
				</button>
			{/if}
			{#if plugin.dev}
				<button
					type="button"
					{@attach tooltip(t.plugins.reloadHint)}
					disabled={busy}
					onclick={() => act(() => api.plugins.reload(handle))}
				>
					<RotateCw size={14} />
					{t.plugins.reload}
				</button>
			{/if}
			{#if listed}
				<button type="button" disabled={busy} onclick={() => act(() => installListed(handle))}>
					{t.plugins.updateTo({ version: listed })}
				</button>
			{:else if local}
				<button type="button" disabled={busy} onclick={() => act(() => api.plugins.update(handle))}>
					{t.plugins.updateTo({ version: local })}
				</button>
			{/if}
			{#if canInstall()}
				<button
					type="button"
					class="icon"
					aria-label={t.plugins.removeNamed({ name: plugin.name })}
					{@attach tooltip(t.plugins.remove)}
					disabled={busy}
					onclick={uninstall}
				>
					<Trash2 size={16} />
				</button>
			{/if}
		{/snippet}
		{#snippet logs()}
			<PluginLogs {handle} />
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
