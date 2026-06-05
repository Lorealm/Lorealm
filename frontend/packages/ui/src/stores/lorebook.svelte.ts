import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	CreateLorebookRequest,
	EntryMeta,
	Lorebook,
	LorebookConfig,
	LorebookEntry,
	LorebookMetadata,
	UpdateLorebookRequest
} from '$lib/types/lorebook.js';
import {
	parseConfig,
	parseEntry,
	serializeEntry,
	type ParsedEntry
} from '$lib/weaver/weaverFile.js';
import { SvelteMap } from 'svelte/reactivity';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: string;
}

/** An entry in the editable draft */
export interface DraftEntry extends ParsedEntry {
	snowflake: Snowflake;
}

/** The editable, parsed form of the active lorebook. */
export interface LorebookDraft {
	id: Snowflake;
	name: string;
	description: string;
	config: LorebookConfig;
	entries: DraftEntry[];
}

class LorebookStore {
	private _api = $state<ApiClient>();
	/** Listing-only metadata */
	metadataList = $state<Array<LorebookMetadata>>();

	get #api() {
		if (!this._api) this._api = useApi(); // NOTE: This assumes it's accessed once during page init. Dirty workaround.
		return this._api;
	}

	private loadedLorebooks = new SvelteMap<Snowflake, Lorebook>(new SvelteMap());
	private activeLorebooks = $state<Snowflake[]>([]);

	/** The editable proxy. */
	private _draft = $state<LorebookDraft | null>(null);
	saveStates = new SvelteMap<Snowflake, SaveState>();
	bookSaveState = $state<SaveState>({ status: 'idle' });

	get editingLorebook(): LorebookDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get entries(): DraftEntry[] {
		return this._draft?.entries ?? [];
	}
	get config(): LorebookConfig | null {
		return this._draft?.config ?? null;
	}
	get active(): Snowflake[] {
		return this.activeLorebooks;
	}
	entrySaveState(snowflake: Snowflake): SaveState {
		return this.saveStates.get(snowflake) ?? { status: 'idle' };
	}

	async loadList() {
		this.metadataList = await this.#api.lorebooks.list();
	}

	async getLorebook(id: Snowflake): Promise<Lorebook | undefined> {
		await this.loadLorebook(id);
		return this.loadedLorebooks.get(id);
	}
	async loadLorebook(id: Snowflake) {
		if (this.loadedLorebooks.has(id)) return;
		const book = await this.#api.lorebooks.get(id);
		this.loadedLorebooks.set(id, book);
	}

	async setActive(id: Snowflake) {
		await this.loadLorebook(id);
		this.activeLorebooks.push(id);
		await this.#api.lorebooks.active.set({ ids: this.activeLorebooks }).catch(() => {
			this.activeLorebooks.pop();
		});
	}

	isActive(id: Snowflake | undefined) {
		if (id) {
			return this.activeLorebooks.includes(id);
		}
		return false;
	}

	async loadActive() {
		this.activeLorebooks = (await this._api?.lorebooks.active.get()) ?? [];
	}

	async startEditing(id: Snowflake) {
		await this.loadLorebook(id);
		const base = this.loadedLorebooks.get(id);
		if (!base) return; // load failed
		this.saveStates.clear();
		this._draft = this.#buildDraft(base);
	}

	stopEditing() {
		this._draft = null;
		this.saveStates.clear();
		this.bookSaveState = { status: 'idle' };
	}

	/** Throw away uncommitted draft changes, back to the last confirmed state. */
	discardChanges() {
		const draft = this._draft;
		if (!draft) return;
		const base = this.loadedLorebooks.get(draft.id);
		if (base) this._draft = this.#buildDraft(base);
		this.saveStates.clear();
		this.bookSaveState = { status: 'idle' };
	}

	#buildDraft(base: Lorebook): LorebookDraft {
		return {
			id: base.id,
			name: base.name,
			description: base.description,
			config: parseConfig(base.config_yaml),
			entries: base.entries.map((e) => ({ snowflake: e.id, ...parseEntry(e.source) }))
		};
	}

	async saveEntry(snowflake: Snowflake) {
		const draft = this._draft;
		if (!draft) return;
		const entry = draft.entries.find((e) => e.snowflake === snowflake);
		if (!entry) return;

		let source: string;
		try {
			source = serializeEntry({
				doc: entry.doc,
				meta: $state.snapshot(entry.meta) as EntryMeta,
				body: entry.body
			});
		} catch (e) {
			this.saveStates.set(snowflake, { status: 'error', ...extractError(e) });
			return;
		}

		this.saveStates.set(snowflake, { status: 'saving' });
		try {
			const saved = await this.#api.lorebooks.entries.update(draft.id, snowflake, { source });
			this.#advanceBaselineEntry(draft.id, saved);
			this.saveStates.set(snowflake, { status: 'saved' });
		} catch (e) {
			this.saveStates.set(snowflake, { status: 'error', ...extractError(e) });
		}
	}

	/** Add a blank entry. */
	async addEntry(): Promise<Snowflake | undefined> {
		const draft = this._draft;
		if (!draft) return;
		const created = await this.#api.lorebooks.entries.add(draft.id);
		this.loadedLorebooks.get(draft.id)?.entries.push(created);
		draft.entries.push({ snowflake: created.id, ...parseEntry(created.source) });
		return created.id;
	}

	/** Delete an entry. */
	async deleteEntry(snowflake: Snowflake) {
		const draft = this._draft;
		if (!draft) return;
		const di = draft.entries.findIndex((e) => e.snowflake === snowflake);
		if (di === -1) return;
		const removedDraft = draft.entries[di];
		draft.entries.splice(di, 1);

		const base = this.loadedLorebooks.get(draft.id);
		const bi = base ? base.entries.findIndex((e) => e.id === snowflake) : -1;
		const removedBase = bi !== -1 ? base!.entries[bi] : undefined;
		if (bi !== -1) base!.entries.splice(bi, 1);

		this.saveStates.delete(snowflake);
		try {
			await this.#api.lorebooks.entries.delete(draft.id, snowflake);
		} catch (e) {
			draft.entries.splice(di, 0, removedDraft);
			if (removedBase) base!.entries.splice(bi, 0, removedBase);
			throw e;
		}
	}

	#advanceBaselineEntry(bookId: Snowflake, saved: LorebookEntry) {
		const base = this.loadedLorebooks.get(bookId);
		const i = base?.entries.findIndex((e) => e.id === saved.id) ?? -1;
		if (base && i !== -1) base.entries[i] = saved;
	}

	async saveLorebook() {
		const draft = this._draft;
		if (!draft) return;
		this.bookSaveState = { status: 'saving' };
		try {
			const req: UpdateLorebookRequest = {
				name: draft.name,
				description: draft.description,
				config: $state.snapshot(draft.config) as LorebookConfig
			};
			const saved = await this.#api.lorebooks.update(draft.id, req);
			const base = this.loadedLorebooks.get(draft.id);
			if (base) {
				base.name = saved.name;
				base.description = saved.description;
				base.config_yaml = saved.config_yaml;
			}
			this.bookSaveState = { status: 'saved' };
		} catch (e) {
			this.bookSaveState = { status: 'error', ...extractError(e) };
		}
	}

	async createLorebook(req: CreateLorebookRequest): Promise<Lorebook> {
		const book = await this.#api.lorebooks.create(req);
		this.loadedLorebooks.set(book.id, book);
		return book;
	}

	/** Delete a lorebook. */
	async deleteLorebook(id: Snowflake) {
		const prevLoaded = this.loadedLorebooks.get(id);
		const prevList = this.metadataList ? [...this.metadataList] : undefined;
		const wasEditing = this._draft?.id === id;

		this.loadedLorebooks.delete(id);
		if (this.metadataList) this.metadataList = this.metadataList.filter((m) => m.id !== id);
		if (wasEditing) this.stopEditing();

		try {
			await this.#api.lorebooks.delete(id);
		} catch (e) {
			if (prevLoaded) this.loadedLorebooks.set(id, prevLoaded);
			if (prevList) this.metadataList = prevList;
			throw e;
		}
	}
}

function extractError(e: unknown): { message: string; code?: string } {
	console.log(e);
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

export const lorebookStore = new LorebookStore();
