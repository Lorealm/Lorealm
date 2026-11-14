<script lang="ts">
	import { t } from '#lib/stores/i18n.svelte.js';
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { Download, FileCode2, Plus, Trash2, Upload } from '@lucide/svelte';
	import { useApi, usePlatform } from '#lib/api/client.js';
	import { showApiError } from '#lib/stores/errorActions.js';
	import { blueprintStore } from '#lib/stores/blueprint.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import { realmDraft } from './draft.svelte.js';
	import { panes } from './panes.svelte.js';
	import { toWtn } from './serialize.js';
	import { ensureLoaded, itemsOf, RESOURCE_KINDS } from './resources.svelte.js';
	import type { RealmResourceKind } from '#lib/types/realm.js';
	import ImportBlueprintDialog from './ImportBlueprintDialog.svelte';

	let busy = $state(false);
	let fileEl = $state<HTMLInputElement>();
	/** The file whose contents are being shown before importing. */
	let importing = $state<File | null>(null);

	$effect(() => {
		void blueprintStore.loadList();
		// Names for the resource bindings an outline comes back with.
		ensureLoaded(RESOURCE_KINDS);
	});

	async function create() {
		if (busy) return;
		busy = true;
		try {
			const name = t.realms.blueprints.untitled;
			const made = await blueprintStore.create(
				name,
				toWtn(
					{ name, description: '', tags: [], director: null, scenarios: [], config: {} },
					[],
					[]
				).source
			);
			if (!made) return;
			start(name, '');
			beginEditing();
		} finally {
			busy = false;
		}
	}

	async function open(id: Snowflake) {
		const result = await blueprintStore.open(id);
		const current = blueprintStore.current;
		if (!current) return;

		start(current.name, current.description);

		if (result?.status === 'ok') realmDraft.load(result.outline, nameOf);
		beginEditing();
	}

	function beginEditing() {
		blueprintStore.beginEditing(
			realmDraft.name,
			toWtn(
				{
					name: realmDraft.name,
					description: realmDraft.description,
					introduction: realmDraft.introduction,
					tags: realmDraft.tags,
					director: realmDraft.director,
					scenarios: realmDraft.scenarios,
					config: realmDraft.config,
					modules: realmDraft.modules
				},
				realmDraft.inputs,
				realmDraft.cast
			).source,
			realmDraft.isEmpty
		);
	}

	function nameOf(kind: RealmResourceKind, id: Snowflake): string {
		return itemsOf(kind).find((item) => item.id === id)?.name ?? '';
	}

	function start(name: string, description: string) {
		realmDraft.reset();
		panes.reset();
		realmDraft.name = name;
		realmDraft.description = description;
	}

	function pick(event: Event & { currentTarget: HTMLInputElement }) {
		importing = event.currentTarget.files?.[0] ?? null;
		event.currentTarget.value = '';
	}

	async function exportFile(event: MouseEvent, id: Snowflake, name: string) {
		event.stopPropagation();
		try {
			const file = await useApi().blueprints.export(id);
			await usePlatform().files.save(file, {
				label: t.realms.blueprints.fileLabel,
				extensions: ['lrlm'],
				suggestedName: `${name.replace(/[^A-Za-z0-9 _-]/g, '_').trim() || t.realms.blueprints.fileFallback}.lrlm`
			});
		} catch (e) {
			showApiError(e, { title: t.realms.blueprints.exportFailed, durationMs: 15000 });
		}
	}

	async function remove(event: MouseEvent, id: Snowflake) {
		event.stopPropagation();
		await blueprintStore.delete(id);
	}
</script>

<div class="section">
	<div class="head">
		<h2>{t.realms.blueprints.title}</h2>
		<button type="button" class="new" onclick={create} disabled={busy}>
			<Plus size={13} />
			{t.realms.blueprints.new}
		</button>
		<button type="button" class="new" onclick={() => fileEl?.click()}>
			<Upload size={13} />
			{t.realms.blueprints.import}
		</button>
		<input
			bind:this={fileEl}
			type="file"
			accept=".lrlm"
			hidden
			style="display:none"
			onchange={pick}
		/>
	</div>

	<div class="grid">
		{#each blueprintStore.summaries as blueprint (blueprint.id)}
			<div
				class="card"
				role="button"
				tabindex="0"
				onclick={() => open(blueprint.id)}
				onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && open(blueprint.id)}
			>
				<div class="cardTop">
					<FileCode2 size={14} />
					<span class="name">{blueprint.name}</span>
					<button
						type="button"
						class="act"
						{@attach tooltip(t.realms.blueprints.exportHint)}
						aria-label={t.realms.blueprints.exportNamed(blueprint.name)}
						onclick={(e) => exportFile(e, blueprint.id, blueprint.name)}
					>
						<Download size={14} />
					</button>
					<button
						type="button"
						class="act del"
						{@attach tooltip(t.realms.blueprints.delete)}
						aria-label={t.realms.cards.deleteNamed(blueprint.name)}
						onclick={(e) => remove(e, blueprint.id)}
					>
						<Trash2 size={14} />
					</button>
				</div>
				{#if blueprint.description}
					<p class="desc">{blueprint.description}</p>
				{/if}
			</div>
		{:else}
			<p class="empty">{t.realms.blueprints.empty}</p>
		{/each}
	</div>
</div>

{#if importing}
	<ImportBlueprintDialog file={importing} onclose={() => (importing = null)} />
{/if}

<style>
	.section {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
		padding: var(--spacing-lg);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-md);
	}
	h2 {
		margin: 0;
		font-family: var(--font-family-heading);
		font-size: var(--font-size-h4);
		font-weight: var(--font-weight-semibold);
	}
	.new {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		background-color: var(--color-bg-elevated-3);
		border: none;
		border-radius: var(--radius-sm);
		color: var(--color-text-secondary);
		font-size: var(--font-size-sm);
	}
	.new:hover:not(:disabled) {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--spacing-md);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-md);
		background-color: var(--color-bg-panel);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md);
		color: var(--color-text-secondary);
	}
	.card:hover {
		border-color: var(--color-border-active);
		color: var(--color-text-primary);
	}
	.cardTop {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		min-width: 0;
	}
	.name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-md);
	}
	.act {
		flex: 0 0 auto;
		display: flex;
		padding: 2px;
		background: transparent;
		border: none;
		color: var(--color-text-disabled);
	}
	.act:hover {
		color: var(--color-text-accent);
	}
	.del:hover {
		color: var(--color-error-text);
	}
	.desc {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
	.empty {
		margin: 0;
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
	}
</style>
