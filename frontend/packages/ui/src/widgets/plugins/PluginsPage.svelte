<script lang="ts">
	import { onMount } from 'svelte';
	import { Check, FolderInput, Trash2 } from '@lucide/svelte';
	import { useApi } from '$lib/api/client.js';
	import { pluginsStore } from '$lib/stores/plugins.svelte.js';
	import type { InstalledPlugin } from '$lib/types/plugins.js';

	const api = useApi();

	let panelOpen = $state(false);
	let path = $state('');
	let installing = $state(false);
	let buildLog = $state('');
	let error = $state('');

	onMount(() => {
		void pluginsStore.load();
	});

	function closePanel() {
		panelOpen = false;
		path = '';
		buildLog = '';
		error = '';
	}

	/** Runs `change`, then refreshes the list; shows what went wrong, if anything. */
	async function act(change: () => Promise<unknown>) {
		error = '';
		try {
			await change();
			await pluginsStore.reload();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	async function install() {
		if (!path.trim() || installing) return;
		installing = true;
		buildLog = '';
		await act(async () => {
			const report = await api.plugins.install(path.trim());
			buildLog = report.log || `Installed ${report.handle}@${report.version}.`;
		});
		installing = false;
	}

	const hasUpdate = (p: InstalledPlugin) =>
		p.enabled && p.latest !== null && p.latest !== p.version;
</script>

<div class="page">
	<div class="topbar">
		<h2>Plugins</h2>
		<div class="actions">
			<button type="button" class="primary" onclick={() => (panelOpen = true)}>
				<FolderInput size={16} />
				Add from source
			</button>
		</div>
	</div>

	{#if panelOpen}
		<div class="panel">
			<label class="field">
				<span class="label">Folder on the server</span>
				<input type="text" bind:value={path} placeholder="/path/to/plugin" />
				<span class="hint">
					The folder holding the plugin's manifest.toml. Its build commands run on the server, and
					the result becomes available to every user.
				</span>
			</label>

			{#if buildLog}<pre class="log">{buildLog}</pre>{/if}

			<div class="panelActions">
				<button type="button" class="ghost" onclick={closePanel} disabled={installing}>Close</button
				>
				<button
					type="button"
					class="primary"
					onclick={install}
					disabled={!path.trim() || installing}
				>
					{installing ? 'Building…' : 'Install'}
				</button>
			</div>
		</div>
	{/if}

	{#if error}<p class="error">{error}</p>{/if}

	{#if !pluginsStore.loaded}
		<div class="empty">Loading…</div>
	{:else if pluginsStore.plugins.length === 0}
		<div class="empty">No plugins installed yet.</div>
	{:else}
		<div class="list">
			{#each pluginsStore.plugins as plugin (plugin.handle)}
				<div class="item" class:active={plugin.enabled}>
					<div class="meta">
						<span class="name">{plugin.name} <span class="version">{plugin.version}</span></span>
						{#if plugin.description}<span class="description">{plugin.description}</span>{/if}
					</div>
					<div class="rowActions">
						{#if hasUpdate(plugin)}
							<button
								type="button"
								class="outline"
								onclick={() => act(() => api.plugins.update(plugin.handle))}
							>
								Update to {plugin.latest}
							</button>
						{/if}
						<button
							type="button"
							class="outline"
							class:on={plugin.enabled}
							onclick={() => act(() => api.plugins.toggle(plugin.handle, !plugin.enabled))}
						>
							{#if plugin.enabled}<Check size={14} /> Enabled{:else}Enable{/if}
						</button>
						<button
							type="button"
							class="icon"
							aria-label="Remove {plugin.name} for everyone"
							title="Remove for everyone"
							onclick={() => act(() => api.plugins.uninstall(plugin.handle))}
						>
							<Trash2 size={16} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.page {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}

	.topbar {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		padding: var(--spacing-md) var(--spacing-lg);
		border-bottom: 1px solid var(--color-border-subtle);
		flex-shrink: 0;
	}
	.topbar h2 {
		font-size: var(--font-size-h4);
	}
	.actions {
		margin-left: auto;
	}

	button.primary {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}

	.panel {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		margin: var(--spacing-lg);
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
	.ghost {
		background: none;
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
		min-width: 0;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.version,
	.description {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.description {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.rowActions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
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

	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
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
