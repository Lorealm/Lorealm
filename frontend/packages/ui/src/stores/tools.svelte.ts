import { type ApiClient, useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { CreateUserTool, Param, UpdateUserTool, UserTool } from '$lib/types/tools.js';
import { showError } from '$lib/stores/errorToast.svelte.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: string;
}

/**
 * The editable form of a tool. `display_name` uses '' to mean "no value"
 * (same convention as the persona draft); `params` is a deep copy so edits
 * don't mutate the baseline we diff against.
 */
export interface ToolDraft {
	id: Snowflake;
	name: string;
	display_name: string; // '' == none
	description: string;
	params: Param[];
	body: string;
}

// Mirrors the server-side validate_name: 1–64 chars, [A-Za-z0-9_-].
const NAME_RE = /^[a-zA-Z0-9_-]{1,64}$/;

class ToolStore {
	private _api = $state<ApiClient>();
	/** Listing-only metadata; carries `enabled`, which the draft never does. */
	loadedTools = $state<Array<UserTool>>();

	get #api() {
		if (!this._api) this._api = useApi(); // accessed once during page init
		return this._api;
	}

	#baseline = $state<UserTool | null>(null);
	private _draft = $state<ToolDraft | null>(null);

	saveState = $state<SaveState>({ status: 'idle' });

	#saving = false;
	#resaveQueued = false;
	/** Last toasted save error, so autosave doesn't stack identical toasts. */
	#lastError: string | null = null;

	get editingTool(): ToolDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get hasUnsavedChanges(): boolean {
		if (!this.#baseline || !this._draft) return false;
		return Object.keys(diffTool(this.#baseline, this._draft)).length > 0;
	}

	async loadTools() {
		const tools = await this.#api?.tools.list();
		if (tools) this.loadedTools = tools;
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
			body: ''
		};
		try {
			const id = await this.#api.tools.create(create);
			await this.loadTools();
			void this.startEditing(id);
			return id;
		} catch (e) {
			showError(e, { title: 'Create failed' });
		}
	}

	/** Delete a tool. Optimistic; reverts the list if the request fails. */
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

	/** Toggle a tool on/off. Optimistic; reverts the row if the request fails. */
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

		// Block invalid names before they reach the server, so an in-progress
		// rename ('' or a bad char) shows an inline hint instead of toast spam.
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
			if (message !== this.#lastError) {
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
		// Deep copy: the draft's params are mutated in place by the editor, and
		// the baseline must stay pristine for diffTool to detect real changes.
		params: $state.snapshot(t.params),
		body: t.body
	};
}

/**
 * Build a merge-patch. `params` is replace-the-whole-array (the API takes the
 * full Param[]), so it's included only when the array changed at all.
 */
function diffTool(base: UserTool, draft: ToolDraft): UpdateUserTool {
	const patch: UpdateUserTool = {};
	if (draft.name !== base.name) patch.name = draft.name;

	const dn = draft.display_name === '' ? null : draft.display_name;
	if (dn !== (base.display_name ?? null)) patch.display_name = dn;

	if (draft.description !== base.description) patch.description = draft.description;
	if (!paramsEqual(draft.params, base.params)) patch.params = draft.params;
	if (draft.body !== base.body) patch.body = draft.body;
	return patch;
}

/**
 * Structural equality via JSON. Order-sensitive, which is fine: the draft is
 * cloned from the baseline (preserving key/element order), so an untouched
 * param compares equal, and any reorder is a genuine change worth saving.
 */
function paramsEqual(a: Param[], b: Param[]): boolean {
	return JSON.stringify(a) === JSON.stringify(b);
}

function extractError(e: unknown): { message: string; code?: string } {
	if (e instanceof Error) {
		const code = (e as { code?: unknown }).code;
		return { message: e.message, code: typeof code === 'string' ? code : undefined };
	}
	if (e && typeof e === 'object') {
		const err = e as Record<string, unknown>;
		const code = typeof err.error_code === 'string' ? err.error_code : undefined;
		const message = typeof err.message === 'string' ? err.message : JSON.stringify(err);
		return { message, code };
	}
	return { message: String(e) };
}

export const toolStore = new ToolStore();
