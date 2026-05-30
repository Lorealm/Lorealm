import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { ContextNode, NodeContent, NodePath, SourceDefinition } from '$lib/types/node.js';
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
} from '$lib/types/template.js';
import { SvelteMap } from 'svelte/reactivity';

// ---------------------------------------------------------------------------
// Tree helpers (pure). Nodes nest: a group node holds `content.children`.
// A NodePath is the list of indices to walk from the roots. The parent of a
// node at path P is `childrenAt(roots, P.slice(0, -1))`.
// ---------------------------------------------------------------------------

function isGroup(content: NodeContent): content is Extract<NodeContent, { type: 'group' }> {
	return content.type === 'group';
}

/** The children array at `parentPath` ([] = roots). Undefined if the path
 *  doesn't resolve through groups. */
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

/** Detach a $state proxy into a plain, independent copy. */
function clone<T>(value: T): T {
	return structuredClone($state.snapshot(value)) as T;
}

// ---------------------------------------------------------------------------

/// TODO: errors are rolled back locally and re-thrown so pages can react.
/// They are not surfaced/stored on the store itself yet.
class TemplateStore {
	private _api = $state<ApiClient>();
	/** Metadata list over ContextTemplates. Does not contain concrete values. Used for listing only. */
	metadataList = $state<Array<TemplateMetadata>>();

	sources = $state<Array<SourceDefinition>>();

	get #api() {
		if (!this._api) this._api = useApi(); // NOTE: This assumes it's accessed once during page init. Dirty workaround.
		return this._api;
	}

	/** Server-confirmed templates — the "latest correct state" / rollback baseline. */
	private loadedTemplates = new SvelteMap<Snowflake, ContextTemplate>();
	private activeTemplateId = $state<Snowflake | null>(null);

	/** The editable proxy. Edits apply here first; the UI binds to this. */
	private _draft = $state<ContextTemplate | null>(null);

	/* --- Concept accessors (qualified, symmetric) ------------------------- */

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

	/* --- Loading ----------------------------------------------------------- */

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
		this.activeTemplateId = id;
	}

	/* --- Editing session --------------------------------------------------- */

	async createTemplate(req: CreateTemplateRequest): Promise<ContextTemplate> {
		const tpl = await this.#api.templates.create(req);
		this.loadedTemplates.set(tpl.id, clone(tpl));
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

	/** Throw away uncommitted draft changes, back to the latest correct state. */
	discardChanges() {
		this.rollback();
	}

	/** Promote the current draft to the baseline (call on a successful write). */
	private commit() {
		if (this._draft) this.loadedTemplates.set(this._draft.id, clone(this._draft));
	}

	/** Reset the draft from the latest correct state. */
	private rollback() {
		if (!this._draft) return;
		const base = this.loadedTemplates.get(this._draft.id);
		if (base) this._draft = clone(base);
	}

	/** Persist a move that has ALREADY been applied to the draft locally (e.g. by
	 *  drag-and-drop). Unlike moveNode, this does NOT splice the tree on the way in;
	 *  it only confirms with the server, snaps to the authoritative path, and
	 *  commits. Rolls the draft back to the last known-good on failure. */
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

	/* --- Template-level writes -------------------------------------------- */

	/** Update name/description. Optimistic on the draft (if editing this id). */
	async updateTemplate(id: Snowflake, req: UpdateTemplateRequest) {
		const editingThis = this._draft?.id === id;
		if (editingThis) {
			if (req.name !== undefined) this._draft!.name = req.name;
			if (req.description !== undefined) this._draft!.description = req.description;
		}
		try {
			const tpl = await this.#api.templates.update(id, req);
			this.loadedTemplates.set(id, clone(tpl));
			if (editingThis) this._draft = clone(tpl);
		} catch (e) {
			if (editingThis) this.rollback();
			throw e;
		}
	}

	/** Delete a template. Optimistic across caches + list. */
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

	/* --- Node-level writes (operate on the current draft) ----------------- */

	/** Add a node. Pessimistic — the server mints the id and decides the path. */
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

		const enabled = !node.enabled;
		node.enabled = enabled; // optimistic

		// Mirror the backend: turning a node ON inside an invariant group
		// forces its siblings OFF. (Disabling does nothing extra.)
		if (enabled) {
			const parent = nodeAt(draft.nodes, path.slice(0, -1));
			if (parent && isGroup(parent.content) && parent.content.mode === 'invariant') {
				for (const child of parent.content.children) {
					if (child.id !== nodeId) child.enabled = false;
				}
			}
		}

		try {
			const res: ToggleNodeResponse = await this.#api.templates.nodes.toggle(draft.id, nodeId);
			node.enabled = res.enabled; // reconcile
			this.commit();
		} catch (e) {
			this.rollback();
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
			// Snap to the server's resulting path. Assumes new_path indexes the
			// post-move tree (i.e. the array with the node removed from its old spot).
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
