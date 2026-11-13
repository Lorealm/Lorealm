<script lang="ts">
	import { tooltip } from '../../stores/tooltip.svelte.ts';
	import { ChevronRight, Plus, Search } from '@lucide/svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { tick } from 'svelte';
	import type { Snowflake } from '#lib/types/core.js';
	import type { RealmResourceKind } from '#lib/types/realm.js';
	import { navigator } from '#lib/navigation/navigator.js';
	import type { StudioResource } from '#lib/navigation/navigator.js';
	import { clearDrag, setDrag } from './drag.svelte.js';
	import { realmDraft } from './draft.svelte.js';
	import { browseText, KIND_LABEL, leafPaths } from './model.js';
	import { ensureLoaded, itemsOf, RESOURCE_KINDS } from './resources.svelte.js';

	const STUDIO_KINDS: Partial<Record<RealmResourceKind, StudioResource>> = {
		character: 'character',
		lorebook: 'lorebook',
		template: 'template',
		tool: 'tool',
		persona: 'persona',
		widget: 'widget',
		module: 'module'
	};

	let query = $state('');
	let treeEl = $state<HTMLElement>();
	const collapsed = new SvelteSet<string>();

	$effect(() => {
		ensureLoaded(RESOURCE_KINDS);
	});

	$effect(() => {
		const selection = realmDraft.selection;
		if (selection?.kind !== 'resource') return;
		collapsed.delete(selection.resourceKind);
		const target = `${selection.resourceKind}:${selection.id}`;
		tick().then(() =>
			treeEl
				?.querySelector<HTMLElement>(`[data-asset="${target}"]`)
				?.scrollIntoView({ block: 'nearest' })
		);
	});

	const needle = $derived(query.trim().toLowerCase());

	const sections = $derived(
		RESOURCE_KINDS.map((kind) => ({
			kind,
			label: KIND_LABEL[kind],
			items: itemsOf(kind).filter((i) => !needle || i.name.toLowerCase().includes(needle))
		})).filter((s) => s.items.length > 0 || !needle)
	);

	const inputs = $derived(
		realmDraft.inputs.filter((i) => !needle || i.name.toLowerCase().includes(needle))
	);

	function toggle(key: string) {
		if (!collapsed.delete(key)) collapsed.add(key);
	}

	function open(studio: StudioResource | undefined, id: Snowflake) {
		if (studio) navigator.start().studio().resource(studio, id);
	}

	function isOpen(key: string) {
		// A search should reveal what it matched, whatever was folded away.
		return needle !== '' || !collapsed.has(key);
	}
</script>

<div class="browser">
	<div class="search">
		<Search size={13} />
		<input
			class="plain"
			type="text"
			bind:value={query}
			placeholder="Search assets…"
			aria-label="Search assets"
			autocomplete="off"
			spellcheck="false"
		/>
	</div>

	<div class="tree" bind:this={treeEl}>
		<div class="section">
			<button
				type="button"
				class="head"
				aria-expanded={isOpen('inputs')}
				onclick={() => toggle('inputs')}
			>
				<span class="chevron" class:open={isOpen('inputs')}><ChevronRight size={12} /></span>
				<span class="title">Inputs</span>
				<span class="count">{realmDraft.inputs.length}</span>
			</button>
			<button
				type="button"
				class="add"
				{@attach tooltip('Declare an input')}
				aria-label="Declare an input"
				onclick={() => realmDraft.addInput()}
			>
				<Plus size={12} />
			</button>
		</div>

		{#if isOpen('inputs')}
			{#each inputs as decl (decl.name)}
				<button
					type="button"
					class="item input"
					class:selected={realmDraft.isSelected({ kind: 'input', name: decl.name })}
					draggable="true"
					ondragstart={(e) => setDrag(e, { drag: 'input', name: decl.name, path: [] })}
					ondragend={clearDrag}
					onclick={() => realmDraft.select({ kind: 'input', name: decl.name })}
				>
					<span class="name">{decl.name}</span>
					<span class="hint">{browseText(decl.ty)}</span>
				</button>

				{#if realmDraft.isSelected({ kind: 'input', name: decl.name })}
					{#each leafPaths(decl.ty) as leaf (leaf.path.join('.'))}
						<button
							type="button"
							class="item input leaf"
							draggable="true"
							ondragstart={(e) => setDrag(e, { drag: 'input', name: decl.name, path: leaf.path })}
							ondragend={clearDrag}
							onclick={() => realmDraft.pingField(decl.name, leaf.path)}
						>
							<span class="name">.{leaf.path.join('.')}</span>
							<span class="hint">{leaf.declared}</span>
						</button>
					{/each}
				{/if}
			{:else}
				<p class="empty">
					{needle ? 'No matching inputs.' : 'No inputs. A document with none is a realm.'}
				</p>
			{/each}
		{/if}

		{#each sections as section (section.kind)}
			<div class="section">
				<button
					type="button"
					class="head"
					aria-expanded={isOpen(section.kind)}
					onclick={() => toggle(section.kind)}
				>
					<span class="chevron" class:open={isOpen(section.kind)}>
						<ChevronRight size={12} />
					</span>
					<span class="title">{section.label}</span>
					<span class="count">{section.items.length}</span>
				</button>
			</div>

			{#if isOpen(section.kind)}
				{#each section.items as item (item.id)}
					{@const studio = STUDIO_KINDS[section.kind]}
					<button
						type="button"
						class="item"
						class:selected={realmDraft.isSelected({
							kind: 'resource',
							resourceKind: section.kind,
							id: item.id,
							name: item.name
						})}
						data-asset="{section.kind}:{item.id}"
						draggable="true"
						ondragstart={(e) =>
							setDrag(e, {
								drag: 'resource',
								kind: section.kind,
								id: item.id,
								name: item.name
							})}
						ondragend={clearDrag}
						onclick={() =>
							realmDraft.select({
								kind: 'resource',
								resourceKind: section.kind,
								id: item.id,
								name: item.name
							})}
						ondblclick={() => open(studio, item.id)}
						onkeydown={(e) => e.key === 'Enter' && open(studio, item.id)}
						{@attach tooltip(
							studio ? 'Drag onto a field, or double-click to open' : 'Drag onto a field'
						)}
					>
						<span class="name">{item.name}</span>
						{#if item.hint}<span class="hint">{item.hint}</span>{/if}
					</button>
				{:else}
					<p class="empty">Nothing here yet.</p>
				{/each}
			{/if}
		{/each}
	</div>
</div>

<style>
	.browser {
		display: flex;
		flex-direction: column;
		width: 100%;
		min-height: 0;
	}
	.search {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
		border-bottom: 1px solid var(--color-border-subtle);
		color: var(--color-text-muted);
	}
	.search input {
		flex: 1 1 auto;
		min-width: 0;
		background: transparent;
		border: none;
		font-size: var(--font-size-sm);
	}

	.tree {
		display: flex;
		flex-direction: column;
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		padding-bottom: var(--spacing-sm);
	}

	.section {
		display: flex;
		align-items: center;
		position: sticky;
		top: 0;
		z-index: 1;
		background-color: var(--color-bg-elevated);
	}
	.head {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-xs) var(--spacing-sm);
		background: transparent;
		border: none;
		color: var(--color-text-secondary);
		text-align: left;
	}
	.head:hover {
		color: var(--color-text-primary);
	}
	.chevron {
		display: flex;
		transition: transform 120ms ease;
	}
	.chevron.open {
		transform: rotate(90deg);
	}
	.title {
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		text-transform: uppercase;
	}
	.count {
		margin-left: auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.add {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		padding: var(--spacing-xs);
		margin-right: var(--spacing-xs);
		background: transparent;
		border: none;
		color: var(--color-text-muted);
		border-radius: var(--radius-sm);
	}
	.add:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}

	.item {
		display: flex;
		align-items: baseline;
		gap: var(--spacing-sm);
		width: 100%;
		padding: 3px var(--spacing-sm) 3px var(--spacing-xl);
		background: transparent;
		border: none;
		color: var(--color-text-secondary);
		text-align: left;
		cursor: grab;
	}
	.item:hover {
		background-color: var(--color-hover);
		color: var(--color-text-primary);
	}
	.item.selected {
		background-color: var(--color-selected);
		color: var(--color-text-accent);
	}
	.item.input .name {
		font-family: var(--font-family-mono);
	}
	.item.leaf {
		padding-left: var(--spacing-3xl);
	}
	.item.leaf .name {
		color: var(--color-text-muted);
	}
	.name {
		flex: 0 1 auto;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--font-size-sm);
	}
	.hint {
		flex: 0 1 auto;
		max-width: 50%;
		margin-left: auto;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		text-align: right;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
	.empty {
		margin: 0;
		padding: var(--spacing-xs) var(--spacing-sm) var(--spacing-sm) var(--spacing-xl);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}
</style>
