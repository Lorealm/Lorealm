<script lang="ts">
	import { tooltip } from '../../../stores/tooltip.svelte.ts';
	import { showError } from '#lib/stores/errorToast.svelte.js';
	import { templateStore } from '#lib/stores/template.svelte.js';
	import { templateWidgetView } from '#lib/stores/templateWidget.svelte.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { ContextNode, NodeContentGroup } from '#lib/types/node.js';
	import { ChevronDown, CircleDot, Folder, LayoutGrid, ListTree, Plug } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import { SvelteSet } from 'svelte/reactivity';

	const groupOf = (node: ContextNode): NodeContentGroup | null =>
		node.content.type === 'group' ? node.content : null;

	let collapsed = $state(false);
	let trail = $state<Snowflake[]>([]);
	const folded = new SvelteSet<Snowflake>();

	let template = $derived(templateStore.activeTemplate);
	let templateId = $derived(template?.id ?? null);

	let tree = $derived(templateWidgetView.value === 'tree');

	onMount(async () => {
		void templateWidgetView.load();
		await templateStore.loadActive();
		const id = templateStore.activeId;
		if (id) await templateStore.loadTemplate(id);
	});

	// Reset the view when the active template changes.
	$effect(() => {
		void templateId;
		trail = [];
		folded.clear();
	});

	/** Walks {@link trail} as far as it still resolves, yielding the visible level. */
	let view = $derived.by(() => {
		const crumbs: ContextNode[] = [];
		let nodes = template?.nodes ?? [];
		for (const id of trail) {
			const group = nodes.find((n) => n.id === id);
			const content = group ? groupOf(group) : null;
			if (!group || !content) break;
			crumbs.push(group);
			nodes = content.children;
		}
		return { crumbs, nodes };
	});

	// A group can disappear while drilled into it (deleted in Studio).
	$effect(() => {
		if (view.crumbs.length !== trail.length) trail = view.crumbs.map((c) => c.id);
	});

	const flatten = (nodes: ContextNode[]): ContextNode[] =>
		nodes.flatMap((n) => [n, ...flatten(groupOf(n)?.children ?? [])]);

	let counted = $derived(tree ? flatten(template?.nodes ?? []) : view.nodes);
	let enabledCount = $derived(counted.filter((n) => n.enabled).length);

	function toggle(node: ContextNode) {
		templateStore
			.toggleActiveNode(node.id)
			.catch((err) => showError(err, { title: 'Toggle failed' }));
	}

	function onBox(e: MouseEvent, node: ContextNode) {
		if (e.shiftKey) {
			if (groupOf(node)) trail = [...trail, node.id];
			return;
		}
		toggle(node);
	}

	function onRow(e: MouseEvent, node: ContextNode) {
		const onArrow = (e.target as Element).closest('.fold') !== null;
		if (!e.shiftKey && !(onArrow && groupOf(node))) {
			toggle(node);
			return;
		}
		if (!groupOf(node)) return;
		if (!folded.delete(node.id)) folded.add(node.id);
	}

	function onBoxKey(e: KeyboardEvent) {
		if (e.key !== 'Backspace') return;
		e.preventDefault();
		goUp();
	}

	function goUp(e?: MouseEvent) {
		e?.preventDefault();
		if (trail.length) trail = trail.slice(0, -1);
	}
</script>

{#snippet branch(nodes: ContextNode[])}
	{#each nodes as node (node.id)}
		{@const group = groupOf(node)}
		{@const isFolded = folded.has(node.id)}
		<button
			class="row"
			class:on={node.enabled}
			class:group={group !== null}
			aria-pressed={node.enabled}
			aria-expanded={group ? !isFolded : undefined}
			{@attach tooltip(
				group ? 'Click to toggle, shift-click to collapse' : node.description || node.title
			)}
			onclick={(e) => onRow(e, node)}
		>
			<span class="fold" class:collapsed={isFolded}>
				{#if group}<ChevronDown />{/if}
			</span>
			<span class="tick"></span>
			<span class="row-name">{node.title || 'Untitled'}</span>
			<span class="marks">
				{#if group}
					{#if group.mode === 'invariant'}
						<CircleDot />
					{/if}
					<span class="children">{group.children.length}</span>
				{:else if node.content.type === 'source'}
					<Plug />
				{/if}
			</span>
		</button>
		{#if group && !isFolded && group.children.length > 0}
			<div class="branch" class:off={!node.enabled}>
				{@render branch(group.children)}
			</div>
		{/if}
	{/each}
{/snippet}

<div class="widget" class:collapsed>
	<div class="bar">
		<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
			<span class="chev" class:collapsed><ChevronDown /></span>
			<span class="label">Template Nodes</span>
			{#if template}
				<span class="count">{enabledCount}/{counted.length}</span>
			{/if}
		</button>
		{#if template && !collapsed}
			<button
				class="switch"
				aria-label={tree ? 'Show as grid' : 'Show as tree'}
				{@attach tooltip(tree ? 'Show as grid' : 'Show as tree')}
				onclick={() => templateWidgetView.set(tree ? 'grid' : 'tree')}
			>
				{#if tree}<LayoutGrid />{:else}<ListTree />{/if}
			</button>
		{/if}
	</div>

	{#if !collapsed}
		{#if !template}
			<div class="empty">No active template.</div>
		{:else if tree}
			<div class="tree" role="group" aria-label="Template nodes">
				{#if template.nodes.length === 0}
					<div class="empty">Empty template.</div>
				{:else}
					{@render branch(template.nodes)}
				{/if}
			</div>
		{:else}
			<div class="crumbs">
				<button class="crumb" disabled={trail.length === 0} onclick={() => (trail = [])}>
					{template.name || 'Untitled'}
				</button>
				{#each view.crumbs as crumb, i (crumb.id)}
					<span class="sep">/</span>
					<button
						class="crumb"
						disabled={i === trail.length - 1}
						onclick={() => (trail = trail.slice(0, i + 1))}
					>
						{crumb.title || 'Untitled'}
					</button>
				{/each}
			</div>

			<!-- Right-click anywhere in the grid leaves the current group. -->
			<div class="grid" role="group" aria-label="Template nodes" oncontextmenu={goUp}>
				{#if view.nodes.length === 0}
					<div class="empty">Empty group.</div>
				{:else}
					{#each view.nodes as node (node.id)}
						{@const group = groupOf(node)}
						<button
							class="box"
							class:on={node.enabled}
							class:group={group !== null}
							{@attach tooltip(
								group ? 'Click to toggle, shift-click to open' : node.description || node.title
							)}
							onclick={(e) => onBox(e, node)}
							onkeydown={onBoxKey}
						>
							<span class="name">{node.title || 'Untitled'}</span>
							<span class="marks">
								{#if group}
									<Folder />
									<span class="children">{group.children.length}</span>
									{#if group.mode === 'invariant'}
										<CircleDot />
									{/if}
								{:else if node.content.type === 'source'}
									<Plug />
								{/if}
							</span>
						</button>
					{/each}
				{/if}
			</div>
		{/if}
	{/if}
</div>

<style>
	.widget {
		display: flex;
		flex-direction: column;
		flex: 0 0 auto;
		width: 100%;
		height: var(--inspector-widget-normal, 280px);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.widget.collapsed {
		height: auto;
	}
	.widget.collapsed .bar {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.bar {
		display: flex;
		align-items: stretch;
		flex: 0 0 auto;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 1;
		min-width: 0;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		text-align: left;
		cursor: pointer;
	}
	.switch {
		flex: 0 0 auto;
		display: inline-flex;
		align-items: center;
		background-color: transparent;
		border-radius: 0;
		padding: 0 var(--spacing-md) 0 0;
		color: var(--color-text-muted);
		cursor: pointer;
	}
	.switch:hover {
		color: var(--color-text-accent);
	}
	.switch :global(svg) {
		width: 14px;
		height: 14px;
	}
	.label {
		flex: 1;
		min-width: 0;
		color: var(--color-text-secondary);
		font-size: var(--font-size-xs);
		text-transform: uppercase;
		letter-spacing: var(--letter-spacing-uppercase);
	}
	.count {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-family: var(--font-family-mono);
		font-size: var(--font-size-xs);
	}
	.chev {
		flex: 0 0 auto;
		display: inline-flex;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev :global(svg) {
		width: 14px;
		height: 14px;
	}

	.crumbs {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-md);
		overflow-x: auto;
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.crumb {
		flex: 0 0 auto;
		max-width: 90px;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		background-color: transparent;
		border-radius: var(--radius-sm);
		padding: 0 var(--spacing-xs);
		color: var(--color-text-accent);
		font-size: var(--font-size-xs);
		cursor: pointer;
	}
	.crumb:disabled {
		color: var(--color-text-muted);
		cursor: default;
	}
	.sep {
		flex: 0 0 auto;
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
	}

	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
		gap: var(--spacing-xs);
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-sm) var(--spacing-md);
		align-content: start;
	}
	.tree {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.branch {
		margin-left: 11px;
		padding-left: var(--spacing-xs);
		border-left: 1px solid var(--color-border-subtle);
	}
	.branch.off {
		opacity: 0.5;
	}
	.row {
		display: flex;
		align-items: center;
		flex: 0 0 auto;
		gap: var(--spacing-xs);
		width: 100%;
		padding: 2px var(--spacing-xs);
		background-color: transparent;
		border-radius: var(--radius-sm);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-align: left;
		cursor: pointer;
		user-select: none;
	}
	.row.on {
		color: var(--color-text-primary);
	}
	.row:hover {
		background-color: var(--color-hover);
	}
	.row.group {
		font-family: var(--font-family-heading);
	}
	.fold {
		flex: 0 0 auto;
		display: inline-flex;
		width: 12px;
		height: 12px;
		color: var(--color-text-muted);
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.fold:hover {
		color: var(--color-text-accent);
	}
	.fold.collapsed {
		transform: rotate(-90deg);
	}
	.fold :global(svg) {
		width: 12px;
		height: 12px;
	}
	.tick {
		flex: 0 0 auto;
		width: 8px;
		height: 8px;
		border: 1px solid var(--color-border-medium);
		border-radius: 2px;
	}
	.row.on .tick {
		border-color: var(--color-border-active);
		background-color: var(--color-border-active);
	}
	.row-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		line-height: var(--line-height-snug);
	}

	.empty {
		grid-column: 1 / -1;
		margin: auto;
		padding: var(--spacing-md);
		color: var(--color-text-muted);
		font-size: var(--font-size-sm);
		text-align: center;
	}

	.box {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		gap: var(--spacing-xs);
		min-height: 56px;
		padding: var(--spacing-sm);
		border: 1px dashed var(--color-border-medium);
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated-2);
		color: var(--color-text-muted);
		font-size: var(--font-size-xs);
		text-align: left;
		cursor: pointer;
		user-select: none;
	}
	.box.on {
		border-style: solid;
		border-color: var(--color-border-active);
		background-color: var(--color-selected);
		color: var(--color-text-primary);
	}
	.box:hover {
		background-color: var(--color-hover);
	}
	.box.group {
		font-family: var(--font-family-heading);
	}

	.name {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		overflow: hidden;
		overflow-wrap: anywhere;
		line-height: var(--line-height-snug);
	}
	.marks {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		color: var(--color-text-muted);
	}
	.marks:empty {
		display: none;
	}
	.marks :global(svg) {
		width: 12px;
		height: 12px;
	}
	.children {
		font-family: var(--font-family-mono);
	}
</style>
