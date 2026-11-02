<script lang="ts">
	import { tooltip } from '../../../../../stores/tooltip.svelte.ts';
	import { getContext } from 'svelte';
	import { flip } from 'svelte/animate';
	import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME, type DndEvent } from 'svelte-dnd-action';
	import { templateStore } from '$lib/stores/template.svelte.js';
	import type { ContextNode, NodeContentGroup, NodePath } from '$lib/types/node.js';
	import type { Snowflake } from '$lib/types/core.js';
	import Self from './TemplateNode.svelte';
	import DropGhost from './DropGhost.svelte';
	import { CircleDot, Pencil, Plug, SquaresSubtract } from '@lucide/svelte';

	const FLIP_MS = 150;
	const DND_KEY = 'template:dnd';
	const MENU_KEY = 'template:menu';

	/** True for the placeholder item svelte-dnd-action injects during a drag. */
	const isShadow = (n: ContextNode) => SHADOW_ITEM_MARKER_PROPERTY_NAME in n;

	// Shared drag controller, provided by TemplateEditor. Kept structural so the
	// two files don't need a shared type module.
	interface DndContext {
		isForbidden(ownerId: Snowflake): boolean;
		onConsider(info: { trigger: string; id: Snowflake }): void;
		onFinalize(arg: {
			parentPath: NodePath;
			items: ContextNode[];
			info: { trigger: string; id: Snowflake };
		}): void;
	}

	let {
		node,
		path,
		selectedId,
		onSelect
	}: {
		node: ContextNode;
		/** This node's live path in the tree. */
		path: NodePath;
		selectedId: Snowflake | null;
		onSelect: (id: Snowflake) => void;
	} = $props();

	const dnd = getContext<DndContext>(DND_KEY);

	interface MenuContext {
		open(node: ContextNode, x: number, y: number): void;
	}
	const menu = getContext<MenuContext>(MENU_KEY);

	const stop = (e: Event) => e.stopPropagation();
	function onContext(e: MouseEvent) {
		e.preventDefault();
		menu.open(node, e.clientX, e.clientY);
	}
	function openKebab(e: MouseEvent) {
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu.open(node, r.left, r.bottom);
	}
	function toggleEnabled(e: Event) {
		e.stopPropagation();
		void templateStore.toggleNode(node.id);
	}

	let collapsed = $state(false);
	function toggleCollapse(e: Event) {
		e.stopPropagation();
		collapsed = !collapsed;
	}

	const ROLE_LABEL: Record<string, string> = {
		system: 'SYS',
		user: 'USR',
		assistant: 'AST'
	};

	const group = $derived(node.content.type === 'group' ? node.content : null);
	const leaf = $derived(node.content.type === 'text' ? node.content : null);
	const role = $derived(leaf?.role ?? 'system');
	const isText = $derived(leaf?.type === 'text');
	const isSource = $derived(node.content.type === 'source');
	const showBadges = $derived(
		(group &&
			((node.content as NodeContentGroup).mode === 'invariant' ||
				(node.content as NodeContentGroup).children.length === 0)) ||
			node.content.type === 'source'
	);

	function select() {
		onSelect(node.id);
	}
	function onKey(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			onSelect(node.id);
		}
	}

	function handleConsider(e: CustomEvent<DndEvent<ContextNode>>) {
		dnd.onConsider({ trigger: e.detail.info.trigger, id: e.detail.info.id as Snowflake });
		const c = node.content;
		if (c.type === 'group') c.children = e.detail.items;
	}
	function handleFinalize(e: CustomEvent<DndEvent<ContextNode>>) {
		const c = node.content;
		if (c.type === 'group') c.children = e.detail.items;
		dnd.onFinalize({
			parentPath: path,
			items: e.detail.items,
			info: { trigger: e.detail.info.trigger, id: e.detail.info.id as Snowflake }
		});
	}
</script>

{#snippet enableBox()}
	<input
		class="enable"
		type="checkbox"
		checked={node.enabled}
		aria-label="Enabled"
		{@attach tooltip('Enabled')}
		onpointerdown={stop}
		onclick={stop}
		onchange={toggleEnabled}
	/>
{/snippet}

{#snippet kebab()}
	<button class="kebab" type="button" aria-label="Actions" onpointerdown={stop} onclick={openKebab}>
		⋯
	</button>
{/snippet}

{#snippet badges()}
	{#if showBadges}
		<div class="badges" class:selected={selectedId === node.id}>
			{#if group?.mode === 'invariant'}
				<span class="badge-item" {@attach tooltip('Invariant')}><CircleDot /></span>
			{/if}
			{#if group && group.children.length === 0}
				<span class="badge-item" {@attach tooltip('Empty')}><SquaresSubtract /></span>
			{/if}
			{#if isSource}
				<span class="badge-item" {@attach tooltip('Dynamic source')}><Plug /></span>
			{/if}
		</div>
	{/if}
	{#if isText && role}
		<span class="role-pill" data-role={role}><Pencil /> {ROLE_LABEL[role]}</span>
	{/if}
{/snippet}

{#if group}
	<div class="node group" class:disabled={!node.enabled}>
		<div
			class="node-row group-row"
			class:selected={selectedId === node.id}
			role="button"
			tabindex="0"
			onclick={select}
			onkeydown={onKey}
			oncontextmenu={onContext}
		>
			{@render enableBox()}
			<button
				class="caret"
				type="button"
				aria-label={collapsed ? 'Expand group' : 'Collapse group'}
				aria-expanded={!collapsed}
				onpointerdown={stop}
				onclick={toggleCollapse}
			>
				<svg
					class="chev"
					class:collapsed
					viewBox="0 0 12 12"
					width="12"
					height="12"
					aria-hidden="true"
				>
					<path
						d="M2 4l4 4 4-4"
						fill="none"
						stroke="currentColor"
						stroke-width="1.5"
						stroke-linecap="round"
						stroke-linejoin="round"
					/>
				</svg>
			</button>
			<div class="title-container">
				<span class="title">{node.title || 'Untitled group'}</span>
				{@render badges()}
			</div>
			{@render kebab()}
		</div>

		{#if !collapsed}
			<div
				class="group-children"
				use:dndzone={{
					items: group.children,
					type: 'template-nodes',
					flipDurationMs: FLIP_MS,
					dropTargetStyle: {},
					dropFromOthersDisabled: dnd.isForbidden(node.id)
				}}
				onconsider={handleConsider}
				onfinalize={handleFinalize}
			>
				{#each group.children as child, j (child.id)}
					<div class="dnd-item" animate:flip={{ duration: FLIP_MS }}>
						{#if isShadow(child)}
							<DropGhost />
						{:else}
							<Self node={child} path={[...path, j]} {selectedId} {onSelect} />
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</div>
{:else}
	<div class="node leaf" class:disabled={!node.enabled}>
		<div
			class="node-row"
			data-role={role}
			class:selected={selectedId === node.id}
			role="button"
			tabindex="0"
			onclick={select}
			onkeydown={onKey}
			oncontextmenu={onContext}
		>
			{@render enableBox()}
			<span class="title">{node.title || 'Untitled'}</span>
			{@render badges()}
			{@render kebab()}
		</div>
	</div>
{/if}

<style>
	.node-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
		border: 1px solid transparent;
		border-radius: var(--radius-md);
		background-color: var(--color-bg-elevated);
		color: var(--color-text-primary);
		font-size: var(--font-size-base);
		cursor: pointer;
		user-select: none;
	}
	.node-row:hover {
		background-color: var(--color-hover);
	}
	.node-row.selected {
		background-color: var(--color-selected);
		border-color: var(--color-border-active);
		color: var(--color-text-accent);
	}
	.node-row[data-role] {
		border-left: 3px solid var(--role-color, transparent);
	}
	.node-row[data-role='system'] {
		--role-color: var(--color-role-system);
	}
	.node-row[data-role='user'] {
		--role-color: var(--color-role-user);
	}
	.node-row[data-role='assistant'] {
		--role-color: var(--color-role-assistant);
	}

	.group-row {
		background-color: var(--color-bg-elevated-2);
		font-family: var(--font-family-heading);
	}
	.caret {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 2px;
		background-color: transparent;
		color: var(--color-text-secondary);
		border-radius: var(--radius-sm);
		margin-left: calc(0 - var(--spacing-sm));
	}
	.badges {
		display: flex;
		justify-content: center;
		align-items: center;
		background-color: var(--color-overlay-light);
		border: 1px solid var(--color-border-subtle);
		padding: var(--spacing-xs);
		border-radius: var(--radius-sm);
		gap: var(--spacing-sm);
	}
	.badges.selected {
		border: 1px solid var(--color-border-active);
	}
	.badges:empty {
		display: none;
	}

	.badge-item {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.badge-item :global(svg) {
		width: 1em;
		height: 1em;
		display: block;
	}

	.caret:hover {
		background-color: var(--color-hover);
		color: var(--color-text-accent);
	}
	.chev {
		display: block;
		transition: transform var(--transition-fast) var(--easing-default);
	}
	.chev.collapsed {
		transform: rotate(-90deg);
	}

	.group-children {
		margin-top: var(--spacing-xs);
		margin-left: var(--spacing-md);
		padding-left: var(--spacing-sm);
		border-left: 1px solid var(--color-border-subtle);
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		/* keep empty groups droppable */
		min-height: var(--spacing-xl);
	}

	.title-container {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		min-width: 0;
	}

	.title {
		flex: 0 1 auto;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.enable {
		width: auto;
		flex: 0 0 auto;
		margin: 0;
		padding: 0;
		cursor: pointer;
		margin-right: var(--spacing-sm);
		width: 1.3em;
		height: 1.3em;
	}

	.kebab {
		flex: 0 0 auto;
		padding: 0 var(--spacing-xs);
		background-color: transparent;
		color: var(--color-text-muted);
		font-size: var(--font-size-md);
		line-height: 1;
		opacity: 0;
		margin-left: auto;
	}
	.node-row:hover .kebab,
	.kebab:focus-visible {
		opacity: 1;
	}
	.kebab:hover {
		color: var(--color-text-accent);
	}

	.node.disabled {
		opacity: 0.5;
	}

	.role-pill {
		display: flex;
		flex-direction: row;
		gap: var(--spacing-xs);
		flex: 0 0 auto;
		font-size: var(--font-size-xs);
		letter-spacing: var(--letter-spacing-uppercase);
		padding: 2px var(--spacing-sm);
		border-radius: var(--radius-full);
		background-color: var(--role-bg);
		color: var(--role-text);
	}
	.role-pill[data-role='system'] {
		--role-bg: var(--color-role-system-bg);
		--role-text: var(--color-role-system-text);
	}
	.role-pill[data-role='user'] {
		--role-bg: var(--color-role-user-bg);
		--role-text: var(--color-role-user-text);
	}
	.role-pill[data-role='assistant'] {
		--role-bg: var(--color-role-assistant-bg);
		--role-text: var(--color-role-assistant-text);
	}
	.role-pill :global(svg) {
		display: block;
		height: var(--font-size-sm);
		width: var(--font-size-sm);
	}
</style>
