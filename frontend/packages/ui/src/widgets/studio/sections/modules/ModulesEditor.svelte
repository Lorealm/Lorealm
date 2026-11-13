<script lang="ts">
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { onDestroy } from 'svelte';
	import { navigator } from '#lib/navigation/navigator.js';
	import { moduleStore } from '#lib/stores/modules.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { CodeModule } from '#lib/types/modules.js';
	import WeaverEditor from '#lib/weaver/editor/WeaverEditor.svelte';
	import FieldShell from '#lib/components/FieldShell.svelte';
	import ListSidebar from '#lib/components/ListSidebar.svelte';

	const SAVE_MS = 500;

	let { selectedId }: { selectedId: Snowflake | null } = $props();

	const draft = $derived(moduleStore.editingModule);
	const save = $derived(moduleStore.saveState);
	const modules = $derived(moduleStore.loadedModules ?? []);

	let query = $state('');
	let sort = $state('name-asc');

	function arrange(list: CodeModule[]): CodeModule[] {
		const needle = query.trim().toLowerCase();
		const out = list.filter((m) => needle === '' || m.name.toLowerCase().includes(needle));
		if (sort === 'name-desc') return out.sort((a, b) => b.name.localeCompare(a.name));
		if (sort === 'enabled') return out.sort((a, b) => Number(b.enabled) - Number(a.enabled));
		return out.sort((a, b) => a.name.localeCompare(b.name));
	}

	const shownModules = $derived(arrange([...modules]));

	let timer: ReturnType<typeof setTimeout> | undefined;
	function requestSave() {
		clearTimeout(timer);
		timer = setTimeout(() => void moduleStore.save(), SAVE_MS);
	}
	onDestroy(() => clearTimeout(timer));

	async function flushPending() {
		clearTimeout(timer);
		await moduleStore.save();
	}

	async function select(id: Snowflake) {
		if (id === selectedId) return;
		await flushPending();
		navigator.start().studio().modules(id);
	}

	async function newModule() {
		await flushPending();
		const id = await moduleStore.createModule();
		if (id) navigator.start().studio().modules(id);
	}

	async function removeModule(id: Snowflake, name: string) {
		if (!confirm(`Delete "${name}"? Anything that calls it will stop working.`)) return;
		clearTimeout(timer);
		await moduleStore.deleteModule(id);
		if (id === selectedId) navigator.start().studio().modules();
	}

	const saveLabel = $derived(
		save.status === 'saving'
			? 'Saving…'
			: save.status === 'error'
				? (save.message ?? 'Error')
				: moduleStore.hasUnsavedChanges
					? 'Unsaved'
					: 'Saved'
	);
</script>

{#snippet hint(text: string)}
	<button type="button" class="hint" {@attach tooltip(text)} aria-label={`Help: ${text}`}>?</button>
{/snippet}

<div class="module-editor">
	<ListSidebar
		title="Modules"
		detailOpen={selectedId !== null}
		oncreate={newModule}
		bind:query
		bind:sort
		sortOptions={[
			{ value: 'name-asc', label: 'Name (A–Z)' },
			{ value: 'name-desc', label: 'Name (Z–A)' },
			{ value: 'enabled', label: 'Enabled first' }
		]}
		searchPlaceholder="Search modules…"
	>
		{#snippet status()}
			{#if draft}
				<div class="save-row" class:error={save.status === 'error'}>
					<span class="dot" data-status={save.status}></span>
					<span class="save-label">{saveLabel}</span>
				</div>
			{/if}
		{/snippet}

		{#if shownModules.length === 0}
			<p class="empty">
				{modules.length === 0 ? 'No modules yet.' : 'No modules match that search.'}
			</p>
		{/if}
		{#each shownModules as module (module.id)}
			<div
				class="module-item"
				class:selected={module.id === selectedId}
				class:disabled={!module.enabled}
			>
				<button class="module-open" type="button" onclick={() => select(module.id)}>
					<span class="module-name">{module.name}</span>
				</button>
				<input
					type="checkbox"
					class="switch enable-toggle"
					checked={module.enabled}
					onchange={() => moduleStore.setEnabled(module.id, !module.enabled)}
					{@attach tooltip(
						module.enabled ? 'Enabled — click to disable' : 'Disabled — click to enable'
					)}
					aria-label={module.enabled ? `Disable ${module.name}` : `Enable ${module.name}`}
				/>
			</div>
		{/each}
	</ListSidebar>

	<section class="col-content">
		{#if draft}
			<div class="pane">
				<div class="pane-head">
					<h1>Edit module</h1>
					<button type="button" class="delete" onclick={() => removeModule(draft.id, draft.name)}>
						Delete
					</button>
				</div>

				<div class="field">
					<label for="m-name">
						Name {@render hint(
							'How templates reach this module: @[name.proc()] calls a proc, {{name:value}} reads a value.'
						)}
					</label>
					<input id="m-name" class="mono" bind:value={draft.name} oninput={requestSave} />
				</div>

				<div class="field">
					<label for="m-description">Description</label>
					<input id="m-description" bind:value={draft.description} oninput={requestSave} />
				</div>

				<FieldShell
					label="Source"
					hint="Values and procs to share with tools, lorebooks and realms while this module is enabled."
				>
					{#snippet editor(fill: boolean)}
						<div class="body-editor" class:fill>
							{#key draft.id}
								<WeaverEditor
									doc={draft.source}
									dialect="module"
									onChange={(v) => {
										draft.source = v;
										requestSave();
									}}
								/>
							{/key}
						</div>
					{/snippet}
				</FieldShell>
			</div>
		{:else}
			<div class="pane empty-pane">
				<p class="empty">Select a module to edit, or create one.</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.module-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
		position: relative;
	}
	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
		overflow-y: auto;
		position: relative;
	}
	.save-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
		min-width: 0;
	}
	.save-row.error {
		color: var(--color-error-text);
	}
	.save-label {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.dot {
		flex: 0 0 auto;
		width: 8px;
		height: 8px;
		border-radius: var(--radius-full);
		background-color: var(--color-text-muted);
	}
	.dot[data-status='saving'] {
		background-color: var(--color-warning);
	}
	.dot[data-status='saved'] {
		background-color: var(--color-success);
	}
	.dot[data-status='error'] {
		background-color: var(--color-error);
	}
	.module-item {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		border-radius: var(--radius-sm);
	}
	.module-item:hover {
		background-color: var(--color-hover);
	}
	.module-item.selected {
		background-color: var(--color-selected);
	}
	.module-item.disabled .module-name {
		color: var(--color-text-muted);
	}
	.module-open {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		align-items: center;
		background: transparent;
		padding: var(--spacing-sm);
		text-align: left;
	}
	.module-name {
		color: var(--color-text-primary);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-sm);
		font-weight: var(--font-weight-semibold);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.enable-toggle {
		flex: 0 0 auto;
		width: 32px;
		height: 18px;
		padding: 2px;
		display: inline-flex;
		align-items: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		transition: background-color 0.15s ease;
	}
	.pane {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		width: 100%;
		max-width: var(--width-content-max);
		margin: 0 auto;
		padding: var(--spacing-xl);
	}
	.pane-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.empty-pane {
		align-items: center;
		justify-content: center;
	}
	.pane h1 {
		font-size: var(--font-size-h3);
		margin: 0;
	}
	.delete {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
		font-size: var(--font-size-sm);
		color: var(--color-error-text);
	}
	.field {
		display: flex;
		flex-direction: column;
	}
	label {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin-top: var(--spacing-xs);
	}
	.mono {
		font-family: var(--font-family-mono);
	}
	.body-editor {
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.body-editor.fill {
		flex: 1 1 auto;
		height: 100%;
	}
	.body-editor.fill > :global(*) {
		flex: 1 1 auto;
		min-height: 0;
	}
	.hint {
		flex: 0 0 auto;
		width: 16px;
		height: 16px;
		padding: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-full);
		background-color: var(--color-bg-elevated-3);
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		line-height: 1;
		cursor: help;
	}
	.hint:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.empty {
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}

	@media (max-width: 768px) {
		.pane {
			padding: var(--spacing-md);
		}
	}
</style>
