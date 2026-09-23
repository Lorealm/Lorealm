import { extractError, type ErrorCode, type ErrorDetails } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
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
import type { EntryRef } from '$lib/weaver/editor/weaver-references.js';
import { settings } from './settings.svelte.js';
import { navigator } from '$lib/navigation/navigator.js';
import { SvelteMap, SvelteSet } from 'svelte/reactivity';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: ErrorCode;
	details?: ErrorDetails;
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
	/** Listing-only metadata */
	metadataList = $state<Array<LorebookMetadata>>();

	get #api() {
		return useApi();
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
		const index = this.activeLorebooks.indexOf(id);
		const wasActive = index !== -1;

		if (wasActive) {
			this.activeLorebooks.splice(index, 1);
		} else {
			await this.loadLorebook(id);
			this.activeLorebooks.push(id);
		}

		await this.#api.lorebooks.active.set({ ids: this.activeLorebooks }).catch(() => {
			if (wasActive) {
				this.activeLorebooks.splice(index, 0, id); // restore at original position
			} else {
				this.activeLorebooks.pop();
			}
		});
	}

	isActive(id: Snowflake | undefined) {
		if (id) {
			return this.activeLorebooks.includes(id);
		}
		return false;
	}

	async loadActive() {
		this.activeLorebooks = (await this.#api.lorebooks.active.get()) ?? [];
	}

	/**
	 * Pull every active book into `loadedLorebooks`.
	 */
	async loadActiveBooks() {
		await Promise.all(
			this.activeLorebooks.map(
				(id) => this.loadLorebook(id).catch(() => {}) // a book we can't load simply offers no ids
			)
		);
	}

	/**
	 * The entry ids a `[[…]]` or `<trigger id="…">` in the book being edited
	 * can resolve to.
	 */
	get referenceableEntries(): EntryRef[] {
		const draft = this._draft;
		const refs: EntryRef[] = draft
			? draft.entries.map((e) => ({ id: e.meta.id, name: e.meta.name }))
			: [];
		if (!settings.current.weaverSuggestCrossBookIds) return refs;

		const seen = new SvelteSet(refs.map((r) => r.id));
		for (const bookId of this.activeLorebooks) {
			if (bookId === draft?.id) continue;
			const book = this.loadedLorebooks.get(bookId);
			if (!book) continue;
			for (const entry of book.entries) {
				if (seen.has(entry.entry_id)) continue;
				seen.add(entry.entry_id);
				refs.push({
					id: entry.entry_id,
					detail: book.name,
					navigable: settings.current.weaverFollowCrossBookIds
				});
			}
		}
		return refs;
	}

	/**
	 * An entry the editor should select once its book is open. Set by
	 * [`navigateToEntry`] and cleared by the editor that consumes it.
	 */
	pendingEntry = $state<Snowflake | null>(null);

	/**
	 * Open the entry `id` refers to, resolving it the way the runtime would.
	 * Switches lorebooks when the id belongs to another active book.
	 */
	navigateToEntry(id: string) {
		const draft = this._draft;
		const local = draft?.entries.find((e) => e.meta.id === id);
		if (local) {
			this.pendingEntry = local.snowflake;
			return;
		}
		if (!settings.current.weaverFollowCrossBookIds) return;

		for (const bookId of this.activeLorebooks) {
			if (bookId === draft?.id) continue;
			const entry = this.loadedLorebooks.get(bookId)?.entries.find((e) => e.entry_id === id);
			if (entry) {
				this.pendingEntry = entry.id;
				navigator.start().studio().lorebooks(bookId);
				return;
			}
		}
	}

	async startEditing(id: Snowflake) {
		await this.loadLorebook(id);
		const base = this.loadedLorebooks.get(id);
		if (!base) return; // load failed
		this.saveStates.clear();
		this._draft = await this.#buildDraft(base);
	}

	stopEditing() {
		this._draft = null;
		this.saveStates.clear();
		this.bookSaveState = { status: 'idle' };
	}

	/** Throw away uncommitted draft changes, back to the last confirmed state. */
	async discardChanges() {
		const draft = this._draft;
		if (!draft) return;
		const base = this.loadedLorebooks.get(draft.id);
		if (base) this._draft = await this.#buildDraft(base);
		this.saveStates.clear();
		this.bookSaveState = { status: 'idle' };
	}

	async #buildDraft(base: Lorebook): Promise<LorebookDraft> {
		return {
			id: base.id,
			name: base.name,
			description: base.description,
			config: await parseConfig(base.config_yaml),
			entries: await Promise.all(
				base.entries.map(async (e) => ({ snowflake: e.id, ...(await parseEntry(e.source)) }))
			)
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
		draft.entries.push({ snowflake: created.id, ...(await parseEntry(created.source)) });
		this.#bumpEntryCount(draft.id, 1);
		return created.id;
	}

	#bumpEntryCount(bookId: Snowflake, delta: number) {
		const meta = this.metadataList?.find((m) => m.id === bookId);
		if (meta) meta.entry_count = Math.max(0, meta.entry_count + delta);
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
		this.#bumpEntryCount(draft.id, -1);
		try {
			await this.#api.lorebooks.entries.delete(draft.id, snowflake);
		} catch (e) {
			draft.entries.splice(di, 0, removedDraft);
			if (removedBase) base!.entries.splice(bi, 0, removedBase);
			this.#bumpEntryCount(draft.id, 1);
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
		// Keep the listing in step so the new book shows up without a refetch.
		this.metadataList = [
			...(this.metadataList ?? []),
			{
				id: book.id,
				owner_id: book.owner_id,
				name: book.name,
				description: book.description,
				entry_count: book.entries.length
			}
		];
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

export const lorebookStore = new LorebookStore();
