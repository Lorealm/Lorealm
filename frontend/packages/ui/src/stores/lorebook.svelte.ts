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

// ---------------------------------------------------------------------------
// Lorebooks differ from templates in three ways that shape this store:
//
//  1. Flat, not a tree. No move/toggle endpoints — ordering is derived from
//     fields (the list sorts), and a toggle is just an `enabled` edit. So none
//     of the path/tree machinery from the template store carries over.
//  2. The wire entry is a raw `.weaver` source string; `UpdateEntryRequest` is
//     the whole source. The store parses at load (so the form binds to a
//     structured view) and serializes on save, via the tested weaverFile
//     boundary. The book config is parsed for the form but sent back whole as
//     a structured object — the server re-serializes it.
//  3. Autosave must not destroy in-progress work. A failed entry save is
//     almost always a weaver-syntax error in the body, and rolling the draft
//     back would wipe what the author was typing. So entry/config saves do NOT
//     roll back on failure: the draft stands and a per-entry indicator reports
//     "unsaved — invalid" with the server's diagnostic. Because the patch is
//     the full source, an entry with a broken body cannot persist ANY field
//     (an `enabled` toggle included) until the syntax is fixed — that falls
//     out naturally. The only rollback left is a failed delete.
// ---------------------------------------------------------------------------

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	/** Server diagnostic, surfaced verbatim on `error` (weaver parse errors are
	 *  multi-line with carets, so render in a <pre>). */
	message?: string;
	code?: string;
}

/** An entry in the editable draft: a stable storage id plus the parsed view the
 *  form binds to. `doc` is the fidelity anchor for serialize; `meta.id` is the
 *  ContextWeaver id shown in the list. */
export interface DraftEntry extends ParsedEntry {
	snowflake: Snowflake;
}

/** The editable, parsed form of the active lorebook. Edits land here first
 *  (the UI binds to it); the baseline in `loadedLorebooks` is the rollback /
 *  last-confirmed reference. */
export interface LorebookDraft {
	id: Snowflake;
	name: string;
	description: string;
	config: LorebookConfig;
	entries: DraftEntry[];
}

class LorebookStore {
	private _api = $state<ApiClient>();
	/** Listing-only metadata. Does not carry entries or config. */
	metadataList = $state<Array<LorebookMetadata>>();

	get #api() {
		if (!this._api) this._api = useApi(); // accessed once during page init
		return this._api;
	}

	/** Server-confirmed lorebooks (raw wire shape) — the rollback baseline. */
	private loadedLorebooks = new SvelteMap<Snowflake, Lorebook>();

	/** The editable proxy. */
	private _draft = $state<LorebookDraft | null>(null);

	/** Per-entry save/validity, keyed by storage id. The open editor reads its
	 *  entry's state; the list can badge entries left in `error`. */
	saveStates = new SvelteMap<Snowflake, SaveState>();

	/** Book-level (config + name/description) save state. */
	bookSaveState = $state<SaveState>({ status: 'idle' });

	/* --- accessors -------------------------------------------------------- */

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
	entrySaveState(snowflake: Snowflake): SaveState {
		return this.saveStates.get(snowflake) ?? { status: 'idle' };
	}

	/* --- loading ---------------------------------------------------------- */

	async loadList() {
		this.metadataList = await this.#api.lorebooks.list();
	}

	async loadLorebook(id: Snowflake) {
		if (this.loadedLorebooks.has(id)) return;
		const book = await this.#api.lorebooks.get(id);
		this.loadedLorebooks.set(id, book);
	}

	/* --- editing session -------------------------------------------------- */

	/** Open `id` for editing: parse the config and every entry up front (the
	 *  list needs each entry's meta to sort and label). Throws if an entry's
	 *  stored source fails to parse — that should not happen, since the server
	 *  validates on write, but we surface it rather than silently dropping. */
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

	/* --- entry writes (operate on the current draft) ---------------------- */

	/** Serialize the draft entry and persist it. Optimistic field edits already
	 *  live on the draft (the form binds to `meta`/`body`); this only confirms
	 *  with the server. On failure the draft is left untouched and the error is
	 *  recorded for the indicator. Used for every entry mutation, toggles
	 *  included (flip `meta.enabled`, then call this). */
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
			// keep the draft as-is; the author may still be typing
			this.saveStates.set(snowflake, { status: 'error', ...extractError(e) });
		}
	}

	/** Add a blank entry. Pessimistic — the server mints the id and the initial
	 *  (unique) ContextWeaver id. Returns the new storage id so the caller can
	 *  select it. */
	async addEntry(): Promise<Snowflake | undefined> {
		const draft = this._draft;
		if (!draft) return;
		const created = await this.#api.lorebooks.entries.add(draft.id);
		this.loadedLorebooks.get(draft.id)?.entries.push(created);
		draft.entries.push({ snowflake: created.id, ...parseEntry(created.source) });
		return created.id;
	}

	/** Delete an entry. Optimistic; restores on failure. */
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

	/* --- book-level writes ------------------------------------------------ */

	/** Persist name/description/config from the draft. The config goes back as a
	 *  structured object (server re-serializes); extension keys survive because
	 *  the whole parsed object is passed through. No rollback on failure. */
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

	/** Delete a lorebook. Optimistic across caches + list. */
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

/** Unwrap an http rejection into the server's `{ error_code, message }`. The
 *  transport throws an `ApiError` (extends Error) carrying a clean `message` and
 *  optional `code`; the object/string branches are a fallback for raw
 *  network-layer rejections that don't go through that path. */
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
