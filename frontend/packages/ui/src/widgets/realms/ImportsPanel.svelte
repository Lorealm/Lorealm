<script lang="ts">
	import { characterStore } from '$lib/stores/character.svelte.js';
	import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
	import { realmStore } from '$lib/stores/realm.svelte.js';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import { toolStore } from '$lib/stores/tools.svelte.js';
	import type { ImportKind, ImportRule } from '$lib/types/realm.js';
	import type { Snowflake } from '$lib/types/core.js';
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import { studioStore } from '$lib/stores/studio.svelte.js';
	import { SquarePen, TriangleAlert, X } from '@lucide/svelte';
	import { onMount } from 'svelte';

	const KINDS: Array<{ kind: ImportKind; label: string }> = [
		{ kind: 'character', label: 'Characters' },
		{ kind: 'lorebook', label: 'Lorebooks' },
		{ kind: 'template', label: 'Context templates' },
		{ kind: 'tool', label: 'Tools' }
	];

	let busy = $state(false);

	onMount(async () => {
		await Promise.all([
			characterStore.metadataList ? Promise.resolve() : characterStore.loadList(),
			lorebookStore.metadataList ? Promise.resolve() : lorebookStore.loadList(),
			templateStore.metadataList ? Promise.resolve() : templateStore.loadList(),
			toolStore.loadedTools ? Promise.resolve() : toolStore.loadTools()
		]);
	});

	const imports = $derived(realmStore.realm?.definition.imports ?? []);
	const missing = $derived(realmStore.missingImports);

	function options(kind: ImportKind): Array<{ id: Snowflake; name: string }> {
		switch (kind) {
			case 'character':
				return (characterStore.metadataList ?? []).map((c) => ({
					id: c.id,
					name: c.display_name ?? c.name
				}));
			case 'lorebook':
				return (lorebookStore.metadataList ?? []).map((l) => ({ id: l.id, name: l.name }));
			case 'template':
				return (templateStore.metadataList ?? []).map((t) => ({ id: t.id, name: t.name }));
			case 'tool':
				return (toolStore.loadedTools ?? []).map((t) => ({
					id: t.id,
					name: t.display_name ?? t.name
				}));
		}
	}

	function nameFor(kind: ImportKind, id: Snowflake): string {
		return options(kind).find((o) => o.id === id)?.name ?? `#${id}`;
	}

	function importedIds(kind: ImportKind): Snowflake[] {
		return imports.filter((r) => r.kind === kind).map((r) => r.resource_id);
	}

	function addable(kind: ImportKind): Array<{ id: Snowflake; name: string }> {
		const have = new Set(importedIds(kind));
		return options(kind).filter((o) => !have.has(o.id));
	}

	async function save(next: ImportRule[]) {
		if (busy) return;
		busy = true;
		try {
			await realmStore.setImports(next);
		} catch (e) {
			showError(e instanceof Error ? e.message : String(e), {
				title: 'Failed to update imports',
				durationMs: 5000
			});
		} finally {
			busy = false;
		}
	}

	async function add(kind: ImportKind, id: Snowflake) {
		if (!id) return;
		await save([...imports, { kind, resource_id: id }]);
	}

	async function remove(rule: ImportRule) {
		const users = realmStore.importUsers(rule.kind, rule.resource_id);
		if (users.length > 0) {
			showError(
				`Still used by ${users.map((a) => `"${a.name}"`).join(', ')} — unbind it there first.`,
				{ title: 'Import in use', durationMs: 5000 }
			);
			return;
		}
		await save(imports.filter((r) => !(r.kind === rule.kind && r.resource_id === rule.resource_id)));
	}

	async function addMissing() {
		await save([...imports, ...missing]);
	}
</script>

<section class="panel">
	<header>
		<h3>Imports</h3>
		<span class="sub">
			The membrane: only imported resources exist inside the realm. Agents can
			only wear imported characters, use imported templates and tools, and read
			imported lorebooks.
		</span>
	</header>

	{#if missing.length > 0}
		<div class="missing">
			<TriangleAlert size={14} />
			<div class="missingBody">
				<span>
					Agents reference {missing.length}
					{missing.length === 1 ? 'resource' : 'resources'} that
					{missing.length === 1 ? "isn't" : "aren't"} imported yet:
				</span>
				<ul>
					{#each missing as rule (rule.kind + rule.resource_id)}
						<li>{rule.kind} · {nameFor(rule.kind, rule.resource_id)}</li>
					{/each}
				</ul>
				<button type="button" disabled={busy} onclick={addMissing}>Import them</button>
			</div>
		</div>
	{/if}

	{#each KINDS as { kind, label } (kind)}
		{@const rules = imports.filter((r) => r.kind === kind)}
		{@const candidates = addable(kind)}
		<fieldset>
			<legend>{label}</legend>
			{#each rules as rule (rule.resource_id)}
				<div class="row">
					<span class="name">{nameFor(kind, rule.resource_id)}</span>
					<button
						type="button"
						class="jump"
						title="Edit this {kind} in the Studio"
						onclick={() => studioStore.editResource(kind, rule.resource_id)}
					>
						<SquarePen size={12} />
					</button>
					<button
						type="button"
						class="remove"
						title="Remove from the realm"
						disabled={busy}
						onclick={() => remove(rule)}
					>
						<X size={12} />
					</button>
				</div>
			{:else}
				<div class="hint">Nothing imported.</div>
			{/each}
			{#if candidates.length > 0}
				<select
					disabled={busy}
					value=""
					onchange={(e) => {
						const id = e.currentTarget.value as Snowflake;
						e.currentTarget.value = '';
						void add(kind, id);
					}}
				>
					<option value="" disabled>Import a {kind}…</option>
					{#each candidates as option (option.id)}
						<option value={option.id}>{option.name}</option>
					{/each}
				</select>
			{/if}
		</fieldset>
	{/each}
</section>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
	}

	header {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}

	h3 {
		margin: 0;
		font-size: 0.95rem;
	}

	.sub {
		color: var(--color-text-muted);
		font-size: 0.8rem;
		line-height: 1.4;
	}

	.missing {
		display: flex;
		gap: var(--spacing-sm);
		padding: var(--spacing-sm);
		border: 1px solid var(--color-warning, #d9a125);
		border-radius: var(--radius-md, 8px);
		color: var(--color-text-secondary);
		font-size: 0.85rem;
	}

	.missingBody {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		align-items: flex-start;
	}

	.missingBody ul {
		margin: 0;
		padding-left: var(--spacing-lg);
	}

	fieldset {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		border: 1px solid var(--color-border-subtle);
		border-radius: var(--radius-md, 8px);
		padding: var(--spacing-sm);
		margin: 0;
	}

	legend {
		font-size: 0.8rem;
		color: var(--color-text-secondary);
		padding: 0 var(--spacing-xs);
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.name {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 0.9rem;
	}

	.jump,
	.remove {
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
		color: var(--color-text-muted);
	}

	.jump:hover {
		color: var(--color-text-primary, inherit);
	}

	.remove:hover {
		color: var(--color-danger, #e5484d);
	}

	.hint {
		color: var(--color-text-muted);
		font-size: 0.8rem;
	}
</style>
