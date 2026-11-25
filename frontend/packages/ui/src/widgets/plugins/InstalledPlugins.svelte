<script lang="ts">
	import ConfirmDelete from '#lib/components/ConfirmDelete.svelte';
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { onMount } from 'svelte';
	import { Check, FolderInput, Settings, Trash2 } from '@lucide/svelte';
	import { useApi } from '#lib/api/client.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import { marketplaceStore } from '#lib/stores/marketplace.svelte.js';
	import { pluginsStore } from '#lib/stores/plugins.svelte.js';
	import { configStore } from '#lib/stores/serverConfig.svelte.js';
	import type { InstalledPlugin } from '#lib/types/plugins.js';
	import { canInstall, change, installListed } from './actions.svelte.js';

	const api = useApi();

	let panelOpen = $state(false);
	let location = $state('');
	let file = $state<File | null>(null);
	let dev = $state(false);
	let fileEl = $state<HTMLInputElement>();
	let installing = $state(false);
	let buildLog = $state('');
	let error = $state('');

	onMount(() => {
		void pluginsStore.load();
		// Decides whether installing and removing are offered.
		void configStore.loadPermissions();
		void marketplaceStore.load();
	});

	function closePanel() {
		panelOpen = false;
		location = '';
		file = null;
		dev = false;
		if (fileEl) fileEl.value = '';
		buildLog = '';
		error = '';
	}

	async function act(action: () => Promise<unknown>) {
		error = await change(action);
	}

	const ready = $derived(file !== null || location.trim() !== '');

	async function install() {
		if (!ready || installing) return;
		installing = true;
		buildLog = '';
		await act(async () => {
			const report = file
				? await api.plugins.upload(file)
				: await api.plugins.install(location.trim(), dev);
			buildLog =
				report.log || t.plugins.installedReport({ handle: report.handle, version: report.version });
		});
		installing = false;
	}

	const listedUpdate = (p: InstalledPlugin) =>
		canInstall() ? marketplaceStore.updateFor(p.handle) : null;

	const localUpdate = (p: InstalledPlugin) =>
		p.enabled && p.latest !== null && p.latest !== p.version ? p.latest : null;
</script>

{#snippet manualImport()}
	<button type="button" class="outline" onclick={() => (panelOpen = !panelOpen)}>
		<FolderInput size={14} />
		{t.plugins.addManually}
	</button>
{/snippet}

{#if canInstall()}
	<div class="bar">
		{@render manualImport()}
	</div>
{/if}

{#if panelOpen}
	<div class="panel">
		<label class="field">
			<span class="label">{t.plugins.location}</span>
			<input
				type="text"
				bind:value={location}
				disabled={file !== null}
				placeholder="https://github.com/owner/name"
			/>
			<span class="hint">
				{t.plugins.locationHint}
			</span>
		</label>
		<label class="check">
			<input type="checkbox" bind:checked={dev} disabled={file !== null} />
			<span>
				{t.plugins.dev}
				<span class="hint">{t.plugins.devHint}</span>
			</span>
		</label>
		<label class="field">
			<span class="label">{t.plugins.upload}</span>
			<input
				bind:this={fileEl}
				type="file"
				accept=".zip"
				onchange={(e) => (file = e.currentTarget.files?.[0] ?? null)}
			/>
		</label>

		{#if buildLog}<pre class="log">{buildLog}</pre>{/if}

		<div class="panelActions">
			<button type="button" class="ghost" onclick={closePanel} disabled={installing}
				>{t.common.close}</button
			>
			<button type="button" class="primary" onclick={install} disabled={!ready || installing}>
				{installing ? t.plugins.installing : t.plugins.install}
			</button>
		</div>
	</div>
{/if}

{#if error}<p class="error">{error}</p>{/if}

{#if !pluginsStore.loaded}
	<div class="empty">{t.plugins.loading}</div>
{:else if pluginsStore.plugins.length === 0}
	<div class="empty">
		<span>{t.plugins.noneInstalled}</span>
		<button type="button" class="outline" onclick={() => navigator.start().marketplace()}>
			{t.plugins.browse}
		</button>
		{@render manualImport()}
	</div>
{:else}
	<div class="list">
		{#each pluginsStore.plugins as plugin (plugin.handle)}
			{@const listed = listedUpdate(plugin)}
			{@const local = localUpdate(plugin)}
			<div class="item" class:active={plugin.enabled}>
				<button type="button" class="meta" onclick={() => navigator.start().plugins(plugin.handle)}>
					<span class="name">{plugin.name} <span class="muted">{plugin.version}</span></span>
					{#if plugin.description}<span class="muted description">{plugin.description}</span>{/if}
				</button>
				<div class="rowActions">
					{#if listed}
						<button
							type="button"
							class="outline"
							onclick={() => act(() => installListed(plugin.handle))}
						>
							{t.plugins.updateTo({ version: listed })}
						</button>
					{:else if local}
						<button
							type="button"
							class="outline"
							onclick={() => act(() => api.plugins.update(plugin.handle))}
						>
							{t.plugins.updateTo({ version: local })}
						</button>
					{/if}
					<button
						type="button"
						class="outline"
						class:on={plugin.enabled}
						onclick={() => act(() => api.plugins.toggle(plugin.handle, !plugin.enabled))}
					>
						{#if plugin.enabled}<Check size={14} /> {t.plugins.enabled}{:else}{t.plugins
								.enable}{/if}
					</button>
					{#if plugin.has_settings}
						<button
							type="button"
							class="outline"
							aria-label={t.plugins.settingsNamed({ name: plugin.name })}
							{@attach tooltip(t.plugins.settings)}
							onclick={() => navigator.start().settings().user().plugins(plugin.handle)}
						>
							<Settings size={14} />
						</button>
					{/if}
					{#if canInstall()}
						<ConfirmDelete
							prompt={t.plugins.confirmRemove({ name: plugin.name })}
							action={t.common.remove}
							onConfirm={() => act(() => api.plugins.uninstall(plugin.handle))}
						>
							{#snippet trigger(arm)}
								<button
									type="button"
									class="icon"
									aria-label={t.plugins.removeNamed({ name: plugin.name })}
									{@attach tooltip(t.plugins.remove)}
									onclick={arm}
								>
									<Trash2 size={16} />
								</button>
							{/snippet}
						</ConfirmDelete>
					{/if}
				</div>
			</div>
		{/each}
	</div>
{/if}

<style>
	.bar {
		display: flex;
		justify-content: flex-end;
		flex: 0 0 auto;
		padding: var(--spacing-md) var(--spacing-lg) 0;
	}

	button.primary {
		background: var(--gradient-primary);
		color: var(--color-text-white);
	}
	.outline {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: none;
		border: 1px solid var(--color-border-medium);
	}
	.outline.on {
		color: var(--color-success-text);
		border-color: var(--color-success-border);
	}
	.ghost {
		background: none;
	}
	.icon {
		display: flex;
		align-items: center;
		justify-content: center;
		background: none;
		color: var(--color-text-secondary);
	}
	.icon:hover {
		color: var(--color-error-text);
		background: none;
	}

	.panel {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		margin: var(--spacing-md) var(--spacing-lg) 0;
		padding: var(--spacing-lg);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		flex-shrink: 0;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.hint {
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.check {
		display: flex;
		align-items: flex-start;
		gap: var(--spacing-sm);
		cursor: pointer;
	}
	.check > span {
		display: flex;
		flex-direction: column;
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.log {
		max-height: 240px;
		overflow: auto;
		margin: 0;
		padding: var(--spacing-sm);
		border-radius: var(--radius-sm);
		background-color: var(--color-bg-input);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
		white-space: pre-wrap;
	}
	.panelActions {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-sm);
	}

	.list {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-lg);
	}
	.item {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
		padding: var(--spacing-sm) var(--spacing-md);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated);
	}
	.item.active {
		border-color: var(--color-border-active);
	}
	.meta {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		min-width: 0;
		padding: 0;
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.name,
	.description {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.muted {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.rowActions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
	}

	.empty {
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-md);
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
	}
	.error {
		margin: 0;
		padding: var(--spacing-sm) var(--spacing-lg) 0;
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
</style>
