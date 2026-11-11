import { extractError } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { CodeModule, UpdateModuleRequest } from '$lib/types/modules.js';
import { showError } from '$lib/stores/errorToast.svelte.js';
import { weaverVocabularyStore } from '$lib/stores/weaverVocabulary.svelte.js';
import type { SaveState } from '$lib/stores/tools.svelte.js';

export interface ModuleDraft {
	id: Snowflake;
	name: string;
	description: string;
	source: string;
}

const NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

class ModuleStore {
	loadedModules = $state<Array<CodeModule>>();

	get #api() {
		return useApi();
	}

	#baseline = $state<CodeModule | null>(null);
	private _draft = $state<ModuleDraft | null>(null);

	saveState = $state<SaveState>({ status: 'idle' });

	#saving = false;
	#resaveQueued = false;
	#lastError: string | null = null;

	get editingModule(): ModuleDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get hasUnsavedChanges(): boolean {
		if (!this.#baseline || !this._draft) return false;
		return Object.keys(diffModule(this.#baseline, this._draft)).length > 0;
	}

	async loadModules() {
		const modules = await this.#api?.modules.list();
		if (modules) this.loadedModules = modules;
	}

	async startEditing(id: Snowflake) {
		this.#resetSession();
		if (!this.loadedModules) await this.loadModules();
		const found = this.loadedModules?.find((m) => m.id === id) ?? null;
		if (!found) {
			this.saveState = { status: 'error', message: 'Module not found' };
			return;
		}
		this.#baseline = found;
		this._draft = buildDraft(found);
	}

	stopEditing() {
		this.#resetSession();
	}

	/** Create an empty module with a free default name. */
	async createModule(): Promise<Snowflake | undefined> {
		try {
			const created = await this.#api.modules.create({ name: this.#defaultName() });
			await this.loadModules();
			return created.id;
		} catch (e) {
			showError(e, { title: 'Create failed' });
		}
	}

	async deleteModule(id: Snowflake) {
		const previous = this.loadedModules;
		if (this.loadedModules) this.loadedModules = this.loadedModules.filter((m) => m.id !== id);
		if (this.editingId === id) this.#resetSession();
		try {
			await this.#api.modules.delete(id);
			void weaverVocabularyStore.refresh();
		} catch (e) {
			this.loadedModules = previous;
			showError(e, { title: 'Delete failed' });
		}
	}

	async setEnabled(id: Snowflake, enabled: boolean) {
		const list = this.loadedModules;
		const i = list?.findIndex((m) => m.id === id) ?? -1;
		if (!list || i === -1) return;
		const previous = list[i].enabled;
		list[i].enabled = enabled;
		try {
			await this.#api.modules.update(id, { enabled });
			void weaverVocabularyStore.refresh();
		} catch (e) {
			list[i].enabled = previous;
			showError(e, { title: 'Toggle failed' });
		}
	}

	async save() {
		const draft = this._draft;
		const base = this.#baseline;
		if (!draft || !base) return;

		if (!NAME_RE.test(draft.name)) {
			this.saveState = {
				status: 'error',
				message: 'Name may only use letters, digits and _, and may not start with a digit'
			};
			return;
		}

		if (this.#saving) {
			this.#resaveQueued = true;
			return;
		}

		const patch = diffModule(base, draft);
		if (Object.keys(patch).length === 0) {
			if (this.saveState.status === 'saving') this.saveState = { status: 'saved' };
			return;
		}

		this.#saving = true;
		this.saveState = { status: 'saving' };
		try {
			const saved = await this.#api.modules.update(draft.id, patch);
			if (this.editingId !== draft.id) return; // session moved on mid-save
			this.#baseline = saved;
			this.#syncListEntry(saved);
			this.saveState = { status: 'saved' };
			this.#lastError = null;
			void weaverVocabularyStore.refresh();
		} catch (e) {
			const { message, code } = extractError(e);
			this.saveState = { status: 'error', message, code };
			if (code !== 'module.invalid' && message !== this.#lastError) {
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

	#resetSession() {
		this._draft = null;
		this.#baseline = null;
		this.saveState = { status: 'idle' };
		this.#resaveQueued = false;
		this.#lastError = null;
	}

	#defaultName(): string {
		const taken = new Set((this.loadedModules ?? []).map((m) => m.name));
		if (!taken.has('new_module')) return 'new_module';
		let n = 2;
		while (taken.has(`new_module_${n}`)) n++;
		return `new_module_${n}`;
	}

	#syncListEntry(m: CodeModule) {
		const list = this.loadedModules;
		if (!list) return;
		const i = list.findIndex((x) => x.id === m.id);
		if (i !== -1) list[i] = m;
	}
}

function buildDraft(m: CodeModule): ModuleDraft {
	return { id: m.id, name: m.name, description: m.description, source: m.source };
}

function diffModule(base: CodeModule, draft: ModuleDraft): UpdateModuleRequest {
	const patch: UpdateModuleRequest = {};
	if (draft.name !== base.name) patch.name = draft.name;
	if (draft.description !== base.description) patch.description = draft.description;
	if (draft.source !== base.source) patch.source = draft.source;
	return patch;
}

export const moduleStore = new ModuleStore();
