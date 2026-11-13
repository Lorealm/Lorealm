<script lang="ts">
	import { tooltip } from '../../../../stores/tooltip.svelte.ts';
	import { setContext } from 'svelte';
	import { flip } from 'svelte/animate';
	import {
		dndzone,
		TRIGGERS,
		SHADOW_ITEM_MARKER_PROPERTY_NAME,
		type DndEvent
	} from 'svelte-dnd-action';
	import { navigator } from '#lib/navigation/navigator.js';
	import { templateStore } from '#lib/stores/template.svelte.js';
	import type { ContextNode, NodePath } from '#lib/types/node.js';
	import type { Snowflake } from '#lib/types/core.js';
	import type { CreateNodeRequest } from '#lib/types/template.js';
	import TemplateNode from './nodes/TemplateNode.svelte';
	import NodeEditor from './nodes/NodeEditor.svelte';
	import AddNodeButton from './nodes/AddNodeButton.svelte';
	import CreateNodeModal from './modals/CreateNodeModal.svelte';
	import DropGhost from './nodes/DropGhost.svelte';
	import NodeContextMenu from './nodes/NodeContextMenu.svelte';
	import EditSettingsModal from './modals/EditSettingsModal.svelte';
	import TemplateSettings from './TemplateSettings.svelte';
	import { SvelteSet } from 'svelte/reactivity';

	const FLIP_MS = 150;
	const DND_KEY = 'template:dnd';
	const MENU_KEY = 'template:menu';

	const isShadow = (n: ContextNode) => SHADOW_ITEM_MARKER_PROPERTY_NAME in n;

	const template = $derived(templateStore.editingTemplate);

	let selectedId = $state<Snowflake | null>(null);
	let showSettings = $state(false);
	let saveState = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');

	/** Which create flow is open, and where the node should land. */
	let creating = $state<{ kind: 'text' | 'group' | 'source'; parentPath?: NodePath } | null>(null);

	/** Open right-click menu */
	let menu = $state<{ node: ContextNode; x: number; y: number } | null>(null);

	/** Node currently open in the Edit modal */
	let editingNodeId = $state<Snowflake | null>(null);
	const editNode = $derived(
		editingNodeId && template ? findNode(template.nodes, editingNodeId) : null
	);

	const existingSourcePairs = $derived.by(() => {
		const set = new SvelteSet<string>();
		const tpl = template;
		if (!tpl) return set;
		const walk = (nodes: ContextNode[]) => {
			for (const n of nodes) {
				if (n.content.type === 'source') {
					set.add(`${n.content.source_id}\u0000${n.content.node_id}`);
				} else if (n.content.type === 'group') {
					walk(n.content.children);
				}
			}
		};
		walk(tpl.nodes);
		return set;
	});

	async function handleCreate(req: CreateNodeRequest) {
		const target = creating;
		const full = target?.parentPath ? { ...req, parent_path: target.parentPath } : req;
		await templateStore.addNode(full);
		creating = null;
	}

	setContext(MENU_KEY, {
		open(node: ContextNode, x: number, y: number) {
			menu = { node, x, y };
		}
	});

	function onMenuEdit(node: ContextNode) {
		editingNodeId = node.id;
	}
	function onMenuToggle(node: ContextNode) {
		void templateStore.toggleNode(node.id);
	}
	function onMenuDelete(node: ContextNode) {
		if (selectedId === node.id) selectedId = null;
		void templateStore.deleteNode(node.id);
	}
	function onMenuAddChild(node: ContextNode, kind: 'text' | 'source' | 'group') {
		const tpl = template;
		const parentPath = tpl ? (findPath(tpl.nodes, node.id) ?? undefined) : undefined;
		creating = { kind, parentPath };
	}

	const selectedNode = $derived(
		selectedId && template ? findNode(template.nodes, selectedId) : null
	);

	function select(nodeId: Snowflake) {
		selectedId = nodeId;
		showSettings = false;
	}

	function findNode(nodes: ContextNode[], target: Snowflake): ContextNode | null {
		for (const n of nodes) {
			if (n.id === target) return n;
			if (n.content.type === 'group') {
				const found = findNode(n.content.children, target);
				if (found) return found;
			}
		}
		return null;
	}
	function findPath(
		nodes: ContextNode[],
		target: Snowflake,
		prefix: NodePath = []
	): NodePath | null {
		for (let i = 0; i < nodes.length; i++) {
			const n = nodes[i];
			const here = [...prefix, i];
			if (n.id === target) return here;
			if (n.content.type === 'group') {
				const found = findPath(n.content.children, target, here);
				if (found) return found;
			}
		}
		return null;
	}
	function collectDescendants(node: ContextNode, into: Set<Snowflake>) {
		if (node.content.type !== 'group') return;
		for (const child of node.content.children) {
			into.add(child.id);
			collectDescendants(child, into);
		}
	}
	function pathsEqual(a: NodePath, b: NodePath): boolean {
		return a.length === b.length && a.every((v, i) => v === b[i]);
	}

	// Tracks forbidden drop-zones so invalid operations such as dropping into
	// itself doesn't occur
	let forbidden = $state<Set<Snowflake>>(new Set());
	let startPath: NodePath | null = null;

	const dnd = {
		isForbidden(ownerId: Snowflake): boolean {
			return forbidden.has(ownerId);
		},
		onConsider(info: { trigger: string; id: Snowflake }) {
			if (info.trigger !== TRIGGERS.DRAG_STARTED) return;
			const tpl = template;
			if (!tpl) return;
			startPath = findPath(tpl.nodes, info.id);
			const set = new Set<Snowflake>([info.id]);
			const dragged = findNode(tpl.nodes, info.id);
			if (dragged) collectDescendants(dragged, set);
			forbidden = set;
		},
		onFinalize(arg: {
			parentPath: NodePath;
			items: ContextNode[];
			info: { trigger: string; id: Snowflake };
		}) {
			const draggedId = arg.info.id;
			const landed = arg.items.findIndex((n) => n.id === draggedId);
			// This zone is the drop destination only if the node ended up in it.
			if (landed !== -1) {
				const resultPath: NodePath = [...arg.parentPath, landed];
				const moved = !startPath || !pathsEqual(resultPath, startPath);
				if (moved) {
					void templateStore
						.persistMove(draggedId, {
							new_parent_path: arg.parentPath.length ? arg.parentPath : undefined,
							new_index: landed
						})
						.catch(() => {
							saveState = 'error';
						});
				}
			}
			forbidden = new Set();
			startPath = null;
		}
	};
	setContext(DND_KEY, dnd);

	function onRootConsider(e: CustomEvent<DndEvent<ContextNode>>) {
		dnd.onConsider({ trigger: e.detail.info.trigger, id: e.detail.info.id as Snowflake });
		if (template) template.nodes = e.detail.items;
	}
	function onRootFinalize(e: CustomEvent<DndEvent<ContextNode>>) {
		if (template) template.nodes = e.detail.items;
		dnd.onFinalize({
			parentPath: [],
			items: e.detail.items,
			info: { trigger: e.detail.info.trigger, id: e.detail.info.id as Snowflake }
		});
	}
</script>

<div class="template-editor">
	<section class="col-hierarchy">
		<header class="hierarchy-header">
			<div class="header-row">
				<button
					class="back"
					onclick={() => navigator.start().studio().templates()}
					aria-label="Back to templates">←</button
				>
				<span class="tpl-name" {@attach tooltip(template?.name)}
					>{template?.name ?? 'Loading…'}</span
				>
			</div>
			<div class="header-row">
				<div class="add-controls">
					<button
						class="add-btn"
						class:active={showSettings}
						type="button"
						onclick={() => (showSettings = true)}>Settings</button
					>
					<AddNodeButton onpick={(kind) => (creating = { kind })} />
					<button class="add-btn" type="button" onclick={() => (creating = { kind: 'group' })}>
						＋ Group
					</button>
				</div>
			</div>
			<div
				class="save-indicator"
				class:saving={saveState === 'saving'}
				class:saved={saveState === 'saved'}
				class:error={saveState === 'error'}
			>
				{#if saveState === 'saving'}Saving…{:else if saveState === 'saved'}Saved{:else if saveState === 'error'}Save
					failed{/if}
			</div>
		</header>

		{#if template}
			<div
				class="hierarchy"
				use:dndzone={{
					items: template.nodes,
					type: 'template-nodes',
					flipDurationMs: FLIP_MS,
					dropTargetStyle: {}
				}}
				onconsider={onRootConsider}
				onfinalize={onRootFinalize}
			>
				{#each template.nodes as node, i (node.id)}
					<div class="dnd-item" animate:flip={{ duration: FLIP_MS }}>
						{#if isShadow(node)}
							<DropGhost />
						{:else}
							<TemplateNode {node} path={[i]} {selectedId} onSelect={select} />
						{/if}
					</div>
				{/each}
			</div>
		{/if}
	</section>

	<section class="col-content">
		{#if showSettings && template}
			<TemplateSettings {template} />
		{:else}
			<NodeEditor node={selectedNode} bind:saveState />
		{/if}
	</section>

	{#if creating}
		<CreateNodeModal
			kind={creating.kind}
			sources={templateStore.sources}
			existing={existingSourcePairs}
			onclose={() => (creating = null)}
			oncreate={handleCreate}
		/>
	{/if}

	{#if menu}
		<NodeContextMenu
			node={menu.node}
			x={menu.x}
			y={menu.y}
			onclose={() => (menu = null)}
			onedit={onMenuEdit}
			ontoggle={onMenuToggle}
			ondelete={onMenuDelete}
			onaddchild={onMenuAddChild}
		/>
	{/if}

	{#if editNode}
		<EditSettingsModal node={editNode} onclose={() => (editingNodeId = null)} />
	{/if}
</div>

<style>
	.template-editor {
		display: flex;
		flex: 1 0 0;
		min-height: 0;
	}

	.col-hierarchy {
		flex: 0 0 300px;
		min-width: 0; /* don't let the nowrap name's intrinsic width widen the column */
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-right: 1px solid var(--color-border-subtle);
	}

	.col-content {
		flex: 1 1 auto;
		min-width: 0;
		display: flex;
		min-height: 0;
	}

	.hierarchy-header {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-sm);
		padding: var(--spacing-md);
		border-bottom: 1px solid var(--color-border-subtle);
	}
	.header-row {
		display: flex;
		align-items: center;
		gap: var(--spacing-sm);
	}
	.back {
		flex: 0 0 auto;
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.tpl-name {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--color-text-primary);
		font-family: var(--font-family-heading);
		font-size: var(--font-size-md);
	}
	.add-controls {
		flex: 0 0 auto;
		display: flex;
		gap: var(--spacing-xs);
	}
	.add-btn {
		font-size: var(--font-size-sm);
		padding: var(--spacing-xs) var(--spacing-sm);
	}
	.add-btn.active {
		font-weight: bold;
		color: var(--color-text-primary);
	}
	.add-btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}

	.save-indicator {
		min-height: 1em;
		font-size: var(--font-size-sm);
		color: var(--color-text-muted);
	}
	.save-indicator.saving {
		color: var(--color-warning-text);
	}
	.save-indicator.saved {
		color: var(--color-success-text);
	}
	.save-indicator.error {
		color: var(--color-error-text);
	}

	.hierarchy {
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
		padding: var(--spacing-sm);
	}

	@media (max-width: 768px) {
		.template-editor {
			flex-direction: column;
		}
		.col-hierarchy {
			flex: 0 0 45%;
			border-right: none;
			border-bottom: 1px solid var(--color-border-subtle);
		}
	}
</style>
