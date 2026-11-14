<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { onMount } from 'svelte';
	import { Upload, Check, Trash2, Globe } from '@lucide/svelte';
	import { tokenizerStore } from '#lib/stores/tokenizer.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';

	type ImportMode = 'file' | 'network';

	let panelOpen = $state(false);
	let mode = $state<ImportMode>('file');
	let name = $state('');
	let isPublic = $state(false);
	let source = $state('');
	let file = $state<File | null>(null);
	let fileEl = $state<HTMLInputElement>();

	let busy = $state(false);
	let error = $state('');

	const canImport = $derived(mode === 'file' ? file !== null : source.trim() !== '');

	function openPanel() {
		panelOpen = true;
	}
	function closePanel() {
		panelOpen = false;
		mode = 'file';
		name = '';
		isPublic = false;
		source = '';
		file = null;
		error = '';
	}

	function onPick(e: Event & { currentTarget: HTMLInputElement }) {
		const f = e.currentTarget.files?.[0] ?? null;
		file = f;
		if (f && !name.trim()) name = f.name;
	}

	async function doImport() {
		if (!canImport || busy) return;
		busy = true;
		error = '';
		try {
			if (mode === 'file' && file) {
				await tokenizerStore.importFile(
					{ name: name.trim() || file.name, is_public: isPublic },
					file
				);
			} else if (mode === 'network') {
				await tokenizerStore.importNetwork({
					source: source.trim(),
					name: name.trim() || undefined,
					is_public: isPublic
				});
			}
			closePanel();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			busy = false;
		}
	}

	async function toggleActive(id: Snowflake) {
		error = '';
		try {
			if (tokenizerStore.activeId === id) await tokenizerStore.clearActive();
			else await tokenizerStore.setActive(id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	// Direct delete for now — swap for your confirm/toast flow if you want a guard.
	async function remove(id: Snowflake) {
		error = '';
		try {
			await tokenizerStore.remove(id);
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	onMount(() => {
		if (!tokenizerStore.loaded) tokenizerStore.load();
	});
</script>

<div class="page">
	<div class="topbar">
		<h2>{t.nav.user.tokenizers}</h2>
		<div class="actions">
			<button type="button" class="primary" onclick={openPanel}>
				<Upload size={16} />
				{t.settings.tokenizers.import}
			</button>
		</div>
	</div>

	{#if panelOpen}
		<div class="importPanel">
			<div class="modes">
				<button
					type="button"
					class="tab"
					class:active={mode === 'file'}
					onclick={() => (mode = 'file')}
				>
					{t.settings.tokenizers.file}
				</button>
				<button
					type="button"
					class="tab"
					class:active={mode === 'network'}
					onclick={() => (mode = 'network')}
				>
					{t.settings.tokenizers.network}
				</button>
			</div>

			{#if mode === 'file'}
				<div class="field">
					<span class="label">{t.settings.tokenizers.file}</span>
					<div class="filePick">
						<input bind:this={fileEl} type="file" hidden onchange={onPick} />
						<button type="button" class="ghost" onclick={() => fileEl?.click()}
							>{t.settings.tokenizers.chooseFile}</button
						>
						<span class="filename">{file?.name ?? t.settings.tokenizers.noFile}</span>
					</div>
				</div>
			{:else}
				<div class="field">
					<span class="label">{t.settings.tokenizers.source}</span>
					<input
						type="text"
						bind:value={source}
						placeholder={t.settings.tokenizers.sourcePlaceholder}
					/>
				</div>
			{/if}

			<div class="field">
				<span class="label"
					>{mode === 'file' ? t.settings.tokenizers.name : t.settings.tokenizers.nameOptional}</span
				>
				<input type="text" bind:value={name} placeholder={t.settings.tokenizers.displayName} />
			</div>

			<label class="check">
				<input type="checkbox" bind:checked={isPublic} />
				<span>{t.settings.tokenizers.makePublic}</span>
			</label>

			{#if error}<p class="error">{error}</p>{/if}

			<div class="panelActions">
				<button type="button" class="ghost" onclick={closePanel} disabled={busy}
					>{t.common.cancel}</button
				>
				<button type="button" class="primary" onclick={doImport} disabled={!canImport || busy}>
					{busy ? t.settings.tokenizers.importing : t.settings.tokenizers.import}
				</button>
			</div>
		</div>
	{/if}

	{#if !tokenizerStore.loaded}
		<div class="status">{t.settings.tokenizers.loading}</div>
	{:else if tokenizerStore.tokenizers.length === 0}
		<div class="empty">{t.settings.tokenizers.empty}</div>
	{:else}
		<div class="list">
			{#each tokenizerStore.tokenizers as tok (tok.id)}
				{@const active = tokenizerStore.activeId === tok.id}
				<div class="item" class:active>
					<div class="meta">
						<span class="name">{tok.name}</span>
						{#if tok.is_public}<Globe size={13} class="public" />{/if}
					</div>
					<div class="rowActions">
						<button
							type="button"
							class="activeBtn"
							class:on={active}
							onclick={() => toggleActive(tok.id)}
						>
							{#if active}<Check size={14} /> {t.settings.tokenizers.active}{:else}{t.settings
									.tokenizers.setActive}{/if}
						</button>
						<button
							type="button"
							class="icon"
							aria-label={t.common.delete}
							onclick={() => remove(tok.id)}
						>
							<Trash2 size={16} />
						</button>
					</div>
				</div>
			{/each}
		</div>
		{#if error && !panelOpen}<p class="error listError">{error}</p>{/if}
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

	.topbar button.primary,
	.panelActions button.primary {
		align-items: center;
		justify-content: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: var(--gradient-primary);
		color: var(--color-text-white);
		box-shadow: var(--shadow-md);
	}
	.topbar button.primary:hover,
	.panelActions button.primary:hover {
		color: var(--color-text-white);
		box-shadow: var(--shadow-glow);
	}

	.importPanel {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		margin: var(--spacing-lg);
		padding: var(--spacing-lg);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		flex-shrink: 0;
	}
	.modes {
		display: flex;
		gap: var(--spacing-sm);
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

	.field {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}
	.label {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.filePick {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.filePick .ghost {
		background: none;
		flex: 0 0 auto;
	}
	.filename {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.check {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		cursor: pointer;
	}
	.check span {
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}

	.panelActions {
		display: flex;
		justify-content: flex-end;
		gap: var(--spacing-sm);
	}
	.panelActions .ghost {
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
		align-items: center;
		gap: var(--spacing-xs);
		min-width: 0;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta :global(.public) {
		color: var(--color-text-muted);
		flex: 0 0 auto;
	}
	.rowActions {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		margin-left: auto;
	}
	.activeBtn {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		white-space: nowrap;
		background: none;
		border: 1px solid var(--color-border-medium);
	}
	.activeBtn.on {
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

	.status,
	.empty {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		color: var(--color-text-muted);
		padding: var(--spacing-4xl) var(--spacing-lg);
		text-align: center;
	}
	.error {
		margin: 0;
		color: var(--color-error-text);
		font-size: var(--font-size-sm);
	}
	.listError {
		padding: 0 var(--spacing-lg) var(--spacing-lg);
	}
</style>
