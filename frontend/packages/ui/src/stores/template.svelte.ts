import { useApi } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';
import type { ContextNode, NodeContent, NodePath, SourceDefinition } from '#lib/types/node.js';
import type {
	ContextTemplate,
	CreateNodeRequest,
	CreateTemplateRequest,
	MoveNodeRequest,
	MoveNodeResponse,
	NodeResponse,
	TemplateMetadata,
	ToggleNodeResponse,
	UpdateNodeRequest,
	UpdateTemplateRequest
} from '#lib/types/template.js';
import { SvelteMap } from 'svelte/reactivity';

function isGroup(content: NodeContent): content is Extract<NodeContent, { type: 'group' }> {
	return content.type === 'group';
}

function countNodes(nodes: ContextNode[]): number {
	return nodes.reduce(
		(sum, n) => sum + 1 + (isGroup(n.content) ? countNodes(n.content.children) : 0),
		0
	);
}

function childrenAt(roots: ContextNode[], parentPath: NodePath): ContextNode[] | undefined {
	let nodes = roots;
	for (const idx of parentPath) {
		const node = nodes[idx];
		if (!node || !isGroup(node.content)) return undefined;
		nodes = node.content.children;
	}
	return nodes;
}

/** Depth-first search for a node by id, returning its full path. */
function findPath(
	roots: ContextNode[],
	id: Snowflake,
	prefix: NodePath = []
): NodePath | undefined {
	for (let i = 0; i < roots.length; i++) {
		const node = roots[i];
		const path = [...prefix, i];
		if (node.id === id) return path;
		if (isGroup(node.content)) {
			const found = findPath(node.content.children, id, path);
			if (found) return found;
		}
	}
	return undefined;
}

function nodeAt(roots: ContextNode[], path: NodePath): ContextNode | undefined {
	if (path.length === 0) return undefined;
	const parent = childrenAt(roots, path.slice(0, -1));
	return parent?.[path[path.length - 1]];
}

function removeAt(roots: ContextNode[], path: NodePath): ContextNode | undefined {
	if (path.length === 0) return undefined;
	const parent = childrenAt(roots, path.slice(0, -1));
	if (!parent) return undefined;
	const [removed] = parent.splice(path[path.length - 1], 1);
	return removed;
}

function insertAt(
	roots: ContextNode[],
	parentPath: NodePath,
	index: number,
	node: ContextNode
): boolean {
	const parent = childrenAt(roots, parentPath);
	if (!parent) return false;
	parent.splice(Math.min(index, parent.length), 0, node);
	return true;
}

function clone<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T;
}

/// TODO: errors are rolled back locally and re-thrown so pages can react.
class TemplateStore {
	/** Metadata list over ContextTemplates */
	metadataList = $state<Array<TemplateMetadata>>();

	sources = $state<Array<SourceDefinition>>();

	get #api() {
		return useApi();
	}

	private loadedTemplates = new SvelteMap<Snowflake, ContextTemplate>();
	private activeTemplateId = $state<Snowflake | null>(null);

	/** Editable proxy object */
	private _draft = $state<ContextTemplate | null>(null);

	get editingTemplate(): ContextTemplate | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get activeTemplate(): ContextTemplate | null {
		return this.activeTemplateId ? (this.loadedTemplates.get(this.activeTemplateId) ?? null) : null;
	}
	get activeId(): Snowflake | null {
		return this.activeTemplateId;
	}

	async loadList() {
		await this.#api.templates.list().then((template) => {
			this.metadataList = template;
		});
	}

	async loadTemplate(id: Snowflake) {
		if (this.loadedTemplates.has(id)) return;
		await this.#api.templates.get(id).then((template) => {
			this.loadedTemplates.set(id, template);
		});
	}

	async loadSources() {
		this.sources = await this.#api.templates.sources();
	}

	/** Set the active template, loading it if needed. */
	async setActive(id: Snowflake) {
		await this.loadTemplate(id);
		await this.#api.templates.active.set({ id });
		this.activeTemplateId = id;
	}

	async loadActive() {
		this.activeTemplateId = await this.#api.templates.active.get();
	}

	async createTemplate(req: CreateTemplateRequest): Promise<ContextTemplate> {
		const tpl = await this.#api.templates.create(req);
		this.loadedTemplates.set(tpl.id, clone(tpl));
		// Keep the listing in step so the new template shows up without a refetch.
		this.metadataList = [
			...(this.metadataList ?? []),
			{
				id: tpl.id,
				owner_id: tpl.owner_id,
				name: tpl.name,
				description: tpl.description,
				node_count: countNodes(tpl.nodes),
				version: tpl.version,
				created_at: tpl.created_at,
				updated_at: tpl.updated_at
			}
		];
		return tpl;
	}

	async startEditing(id: Snowflake) {
		await this.loadTemplate(id);
		const base = this.loadedTemplates.get(id);
		if (!base) return; // load failed
		this._draft = clone(base);
	}

	stopEditing() {
		this._draft = null;
	}

	discardChanges() {
		this.rollback();
	}

	private commit() {
		if (this._draft) this.loadedTemplates.set(this._draft.id, clone(this._draft));
	}

	/** Reset the draft from the latest correct state. */
	private rollback() {
		if (!this._draft) return;
		const base = this.loadedTemplates.get(this._draft.id);
		if (base) this._draft = clone(base);
	}

	/** Persist a move that has ALREADY been applied to the draft locally */
	async persistMove(nodeId: Snowflake, move: MoveNodeRequest) {
		const draft = this._draft;
		if (!draft) return;
		try {
			const res: MoveNodeResponse = await this.#api.templates.nodes.move(draft.id, nodeId, move);
			const cur = findPath(draft.nodes, nodeId);
			if (cur) {
				const n = removeAt(draft.nodes, cur)!;
				insertAt(draft.nodes, res.new_path.slice(0, -1), res.new_path[res.new_path.length - 1], n);
			}
			this.commit();
		} catch (e) {
			console.error(e);
			this.rollback();
			throw e;
		}
	}

	/** Update name/description */
	async updateTemplate(id: Snowflake, req: UpdateTemplateRequest) {
		const editingThis = this._draft?.id === id;
		if (editingThis) {
			if (req.name !== undefined) this._draft!.name = req.name;
			if (req.description !== undefined) this._draft!.description = req.description;
		}
		try {
			const tpl = await this.#api.templates.update(id, req);
			this.loadedTemplates.set(id, clone(tpl));
			if (this._draft?.id === id) {
				this._draft.name = tpl.name;
				this._draft.description = tpl.description;
				this._draft.updated_at = tpl.updated_at;
			}
			const meta = this.metadataList?.find((m) => m.id === id);
			if (meta) {
				meta.name = tpl.name;
				meta.description = tpl.description;
				meta.updated_at = tpl.updated_at;
			}
		} catch (e) {
			if (editingThis) this.rollback();
			throw e;
		}
	}

	/** Delete a template */
	async deleteTemplate(id: Snowflake) {
		const prevLoaded = this.loadedTemplates.get(id);
		const prevList = this.metadataList ? [...this.metadataList] : undefined;
		const wasEditing = this._draft?.id === id;
		const wasActive = this.activeTemplateId === id;

		this.loadedTemplates.delete(id);
		if (this.metadataList) this.metadataList = this.metadataList.filter((m) => m.id !== id);
		if (wasEditing) this._draft = null;
		if (wasActive) this.activeTemplateId = null;

		try {
			await this.#api.templates.delete(id);
		} catch (e) {
			if (prevLoaded) {
				this.loadedTemplates.set(id, prevLoaded);
				if (wasEditing) this._draft = clone(prevLoaded);
			}
			if (prevList) this.metadataList = prevList;
			if (wasActive) this.activeTemplateId = id;
			throw e;
		}
	}

	/** Add a node. */
	async addNode(create: CreateNodeRequest) {
		const draft = this._draft;
		if (!draft) return;
		const res: NodeResponse = await this.#api.templates.nodes.add(draft.id, create);
		const parentPath = res.path.slice(0, -1);
		insertAt(draft.nodes, parentPath, res.path[res.path.length - 1], res.node);
		this.commit();
	}

	async updateNode(nodeId: Snowflake, update: UpdateNodeRequest) {
		const draft = this._draft;
		if (!draft) return;
		const path = findPath(draft.nodes, nodeId);
		if (!path) return;
		const node = nodeAt(draft.nodes, path)!;

		// optimistic
		if (update.title !== undefined) node.title = update.title;
		if (update.description !== undefined) node.description = update.description;
		if (update.enabled !== undefined) node.enabled = update.enabled;
		if (update.content !== undefined) node.content = update.content;

		try {
			console.log('saving: ', JSON.stringify(update));
			const res: NodeResponse = await this.#api.templates.nodes.update(draft.id, nodeId, update);
			// reconcile with the authoritative node (update doesn't move it)
			const parent = childrenAt(draft.nodes, res.path.slice(0, -1));
			if (parent) parent[res.path[res.path.length - 1]] = res.node;
			this.commit();
		} catch (e) {
			this.rollback();
			throw e;
		}
	}

	async toggleNode(nodeId: Snowflake) {
		const draft = this._draft;
		if (!draft) return;
		const path = findPath(draft.nodes, nodeId);
		if (!path) return;
		const node = nodeAt(draft.nodes, path)!;

		node.enabled = !node.enabled; // optimistic

		try {
			const res: ToggleNodeResponse = await this.#api.templates.nodes.toggle(draft.id, nodeId);
			// Apply the authoritative delta
			for (const [id, enabled] of Object.entries(res.delta)) {
				const p = findPath(draft.nodes, id);
				if (p) nodeAt(draft.nodes, p)!.enabled = enabled;
			}
			this.commit();
		} catch (e) {
			this.rollback();
			throw e;
		}
	}

	/**
	 * Toggle a node on the active template, without an editing session.
	 */
	async toggleActiveNode(nodeId: Snowflake) {
		const id = this.activeTemplateId;
		const base = id ? this.loadedTemplates.get(id) : undefined;
		if (!id || !base) return;

		const optimistic = clone(base);
		const path = findPath(optimistic.nodes, nodeId);
		if (!path) return;
		const node = nodeAt(optimistic.nodes, path)!;
		node.enabled = !node.enabled;
		this.loadedTemplates.set(id, optimistic);

		try {
			const res: ToggleNodeResponse = await this.#api.templates.nodes.toggle(id, nodeId);
			// Apply the authoritative delta (invariant groups turn siblings off).
			const settled = clone(optimistic);
			for (const [changedId, enabled] of Object.entries(res.delta)) {
				const p = findPath(settled.nodes, changedId);
				if (p) nodeAt(settled.nodes, p)!.enabled = enabled;
			}
			this.loadedTemplates.set(id, settled);

			// Keep an open editor of the same template in sync.
			if (this._draft?.id === id) {
				for (const [changedId, enabled] of Object.entries(res.delta)) {
					const p = findPath(this._draft.nodes, changedId);
					if (p) nodeAt(this._draft.nodes, p)!.enabled = enabled;
				}
			}
		} catch (e) {
			this.loadedTemplates.set(id, base);
			throw e;
		}
	}

	async deleteNode(nodeId: Snowflake) {
		const draft = this._draft;
		if (!draft) return;
		const path = findPath(draft.nodes, nodeId);
		if (!path) return;

		removeAt(draft.nodes, path); // optimistic
		try {
			await this.#api.templates.nodes.delete(draft.id, nodeId);
			this.commit();
		} catch (e) {
			this.rollback();
			throw e;
		}
	}

	async moveNode(nodeId: Snowflake, move: MoveNodeRequest) {
		const draft = this._draft;
		if (!draft) return;
		const path = findPath(draft.nodes, nodeId);
		if (!path) return;

		const removed = removeAt(draft.nodes, path);
		if (!removed) return;
		insertAt(draft.nodes, move.new_parent_path ?? [], move.new_index, removed); // optimistic

		try {
			const res: MoveNodeResponse = await this.#api.templates.nodes.move(draft.id, nodeId, move);
			// Snap to the server's resulting path
			const cur = findPath(draft.nodes, nodeId);
			if (cur) {
				const n = removeAt(draft.nodes, cur)!;
				insertAt(draft.nodes, res.new_path.slice(0, -1), res.new_path[res.new_path.length - 1], n);
			}
			this.commit();
		} catch (e) {
			this.rollback();
			throw e;
		}
	}
}

export const templateStore = new TemplateStore();
