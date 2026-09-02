import { extractError } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	CreateUserTool,
	Param,
	ToolSurface,
	UpdateUserTool,
	UserTool
} from '$lib/types/tools.js';
import { showError } from '$lib/stores/errorToast.svelte.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: string;
}

export interface ToolDraft {
	id: Snowflake;
	name: string;
	display_name: string; // '' == none
	description: string;
	params: Param[];
	body: string;
	surface: ToolSurface;
}

const NAME_RE = /^[a-zA-Z0-9_-]{1,64}$/;

class ToolStore {
	/** Listing-only metadata; carries `enabled`, which the draft never does. */
	loadedTools = $state<Array<UserTool>>();

	get #api() {
		return useApi();
	}

	#baseline = $state<UserTool | null>(null);
	private _draft = $state<ToolDraft | null>(null);

	saveState = $state<SaveState>({ status: 'idle' });

	#saving = false;
	#resaveQueued = false;
	/** Last toasted save error to (partially) prevent auto-save spam */
	#lastError: string | null = null;

	/** A builtin opened for reading. Mutually exclusive with an edit session. */
	private _viewing = $state<ToolDraft | null>(null);

	get editingTool(): ToolDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get viewingTool(): ToolDraft | null {
		return this._viewing;
	}
	get selectedId(): Snowflake | null {
		return this._draft?.id ?? this._viewing?.id ?? null;
	}
	get hasUnsavedChanges(): boolean {
		if (!this.#baseline || !this._draft) return false;
		return Object.keys(diffTool(this.#baseline, this._draft)).length > 0;
	}

	async loadTools() {
		const tools = await this.#api?.tools.list();
		if (tools) this.loadedTools = tools;
	}

	/** Server-shipped builtin tools: bindable in realms without an import, read-only. */
	builtinTools = $state<Array<UserTool>>();

	async loadBuiltins() {
		const tools = await this.#api?.tools.builtins();
		if (tools) this.builtinTools = tools;
	}

	async startEditing(id: Snowflake) {
		this.#resetSession();
		if (!this.loadedTools) await this.loadTools();
		const found = this.loadedTools?.find((t) => t.id === id) ?? null;
		if (!found) {
			this.saveState = { status: 'error', message: 'Tool not found' };
			return;
		}
		this.#baseline = found;
		this._draft = buildDraft(found);
	}

	/** Open a builtin read-only. */
	async viewBuiltin(id: Snowflake) {
		this.#resetSession();
		if (!this.builtinTools) await this.loadBuiltins();
		const found = this.builtinTools?.find((t) => t.id === id) ?? null;
		if (!found) {
			this.saveState = { status: 'error', message: 'Tool not found' };
			return;
		}
		this._viewing = buildDraft(found);
	}

	stopEditing() {
		this.#resetSession();
	}

	discardChanges() {
		if (this.#baseline) this._draft = buildDraft(this.#baseline);
		this.saveState = { status: 'idle' };
	}

	/** Create a blank tool with a non-colliding default name, then open it. */
	async createTool(): Promise<Snowflake | undefined> {
		const create: CreateUserTool = {
			name: this.#defaultName(),
			display_name: null,
			description: '',
			params: [],
			body: '',
			surface: 'tool'
		};
		try {
			const id = await this.#api.tools.create(create);
			await this.loadTools();
			return id;
		} catch (e) {
			showError(e, { title: 'Create failed' });
		}
	}

	/**
	 * Copy a shipped row into one of your own under the same name.
	 */
	async forkBuiltin(id: Snowflake): Promise<Snowflake | undefined> {
		if (!this.builtinTools) await this.loadBuiltins();
		const source = this.builtinTools?.find((t) => t.id === id);
		if (!source) return;

		try {
			const forked = await this.#api.tools.create({
				name: source.name,
				display_name: source.display_name,
				description: source.description,
				params: $state.snapshot(source.params),
				body: source.body,
				surface: source.surface
			});
			await this.loadTools();
			return forked;
		} catch (e) {
			showError(e, { title: 'Could not create your version' });
		}
	}

	async deleteTool(id: Snowflake) {
		const previous = this.loadedTools;
		if (this.loadedTools) this.loadedTools = this.loadedTools.filter((t) => t.id !== id);
		if (this.editingId === id) this.#resetSession();
		try {
			await this.#api.tools.delete(id);
		} catch (e) {
			this.loadedTools = previous;
			showError(e, { title: 'Delete failed' });
		}
	}

	async setEnabled(id: Snowflake, enabled: boolean) {
		const list = this.loadedTools;
		const i = list?.findIndex((t) => t.id === id) ?? -1;
		if (!list || i === -1) return;
		const previous = list[i].enabled;
		list[i].enabled = enabled;
		try {
			await this.#api.tools.toggle(id, { enabled });
		} catch (e) {
			list[i].enabled = previous;
			showError(e, { title: 'Toggle failed' });
		}
	}

	#resetSession() {
		this._draft = null;
		this._viewing = null;
		this.#baseline = null;
		this.saveState = { status: 'idle' };
		this.#resaveQueued = false;
		this.#lastError = null;
	}

	#defaultName(): string {
		const taken = new Set((this.loadedTools ?? []).map((t) => t.name));
		if (!taken.has('new_tool')) return 'new_tool';
		let n = 2;
		while (taken.has(`new_tool_${n}`)) n++;
		return `new_tool_${n}`;
	}

	async save() {
		const draft = this._draft;
		const base = this.#baseline;
		if (!draft || !base) return;

		if (!NAME_RE.test(draft.name)) {
			this.saveState = {
				status: 'error',
				message: 'Name must be 1–64 chars: letters, digits, _ or -'
			};
			return;
		}

		if (this.#saving) {
			this.#resaveQueued = true;
			return;
		}

		const patch = diffTool(base, draft);
		if (Object.keys(patch).length === 0) {
			if (this.saveState.status === 'saving') this.saveState = { status: 'saved' };
			return;
		}

		this.#saving = true;
		this.saveState = { status: 'saving' };
		try {
			const saved = await this.#api.tools.update(draft.id, patch);
			if (this.editingId !== draft.id) return; // session moved on mid-save
			this.#baseline = saved;
			this.#syncListEntry(saved);
			this.saveState = { status: 'saved' };
			this.#lastError = null;
		} catch (e) {
			const { message, code } = extractError(e);
			this.saveState = { status: 'error', message, code };
			// A body that doesn't compile is already underlined in the editor
			// and spelled out in the save row. A toast would be the third copy,
			// raised again on every pause in typing.
			if (code !== 'tool.invalid_body' && message !== this.#lastError) {
				showError(message, { title: 'Save failed' });
				this.#lastError = message;
			}
		} finally {
			this.#saving = false;
			if (this.#resaveQueued) {
				this.#resaveQueued = false;
				void this.save();
			}
		}
	}

	/** Keep the sidebar in sync with the saved name/enabled state. */
	#syncListEntry(t: UserTool) {
		const list = this.loadedTools;
		if (!list) return;
		const i = list.findIndex((x) => x.id === t.id);
		if (i !== -1) list[i] = t;
	}
}

function buildDraft(t: UserTool): ToolDraft {
	return {
		id: t.id,
		name: t.name,
		display_name: t.display_name ?? '',
		description: t.description,
		params: $state.snapshot(t.params),
		body: t.body,
		surface: t.surface
	};
}

function diffTool(base: UserTool, draft: ToolDraft): UpdateUserTool {
	const patch: UpdateUserTool = {};
	if (draft.name !== base.name) patch.name = draft.name;

	const dn = draft.display_name === '' ? null : draft.display_name;
	if (dn !== (base.display_name ?? null)) patch.display_name = dn;

	if (draft.description !== base.description) patch.description = draft.description;
	if (!paramsEqual(draft.params, base.params)) patch.params = draft.params;
	if (draft.body !== base.body) patch.body = draft.body;
	if (draft.surface !== base.surface) patch.surface = draft.surface;
	return patch;
}

function paramsEqual(a: Param[], b: Param[]): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

export const toolStore = new ToolStore();
