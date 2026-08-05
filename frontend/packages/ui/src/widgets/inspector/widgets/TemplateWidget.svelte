<script lang="ts">
	import { showError } from '$lib/stores/errorToast.svelte.js';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import type { Snowflake } from '$lib/types/core.js';
	import type { ContextNode, NodeContentGroup } from '$lib/types/node.js';
	import { ChevronDown, CircleDot, Folder, Plug } from '@lucide/svelte';
	import { onMount } from 'svelte';

	const groupOf = (node: ContextNode): NodeContentGroup | null =>
		node.content.type === 'group' ? node.content : null;

	let collapsed = $state(false);
	/** Ids of the groups drilled into, outermost first. */
	let trail = $state<Snowflake[]>([]);

	let template = $derived(templateStore.activeTemplate);
	let templateId = $derived(template?.id ?? null);

	onMount(async () => {
		await templateStore.loadActive();
		const id = templateStore.activeId;
		if (id) await templateStore.loadTemplate(id);
	});

	// Reset the view when the active template changes.
	$effect(() => {
		void templateId;
		trail = [];
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

	let enabledCount = $derived(view.nodes.filter((n) => n.enabled).length);

	function onBox(e: MouseEvent, node: ContextNode) {
		if (e.shiftKey) {
			if (groupOf(node)) trail = [...trail, node.id];
			return;
		}
		templateStore
			.toggleActiveNode(node.id)
			.catch((err) => showError(err, { title: 'Toggle failed' }));
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

<div class="widget" class:collapsed>
	<button class="head" aria-expanded={!collapsed} onclick={() => (collapsed = !collapsed)}>
		<span class="chev" class:collapsed><ChevronDown /></span>
		<span class="label">Template Nodes</span>
		{#if template}
			<span class="count">{enabledCount}/{view.nodes.length}</span>
		{/if}
	</button>

	{#if !collapsed}
		{#if !template}
			<div class="empty">No active template.</div>
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
							title={group
								? 'Click to toggle, shift-click to open'
								: node.description || node.title}
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
	.widget.collapsed .head {
		border-bottom: none;
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.head {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
		flex: 0 0 auto;
		width: 100%;
		background-color: transparent;
		border-radius: 0;
		padding: var(--spacing-sm) var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
		text-align: left;
		cursor: pointer;
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
