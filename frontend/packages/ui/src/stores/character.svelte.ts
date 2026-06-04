import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type {
	Character,
	CharacterEdit,
	CharacterSummary,
	CharacterVersionInfo,
	EditDefinition,
	EditMetadata
} from '$lib/types/character.js';

// ---------------------------------------------------------------------------
// Characters differ from lorebooks in the one way that reshapes the whole store:
//
//  1. The wire write is an ordered `CharacterEdit[]`, NOT a whole-object PUT,
//     and a partial failure persists preceding ops. So instead of saving a
//     serialized blob, the store diffs the baseline against the draft into a
//     minimal op list and sends that.
//  2. Every endpoint returns the full `Character`, so optimistic mutation buys
//     nothing. The draft is still the source of truth (it's what the form binds
//     to), but we never roll it back. On success the baseline simply advances;
//     the draft is left alone. The only thing pulled back onto the draft is the
//     server-minted ids for newly added greetings/examples — patched by object
//     reference so re-diffing won't re-add them, and without touching any text
//     the author may still be editing.
//  3. On a (possibly partial) failure we refetch the character to re-base,
//     reconcile ids the same way, and surface the diagnostic. The next diff
//     then retries only whatever still differs.
//
// A single book-style `saveState` covers the whole character (one PUT, one
// status). An in-flight guard coalesces concurrent saves; a post-save diff
// check re-runs the save if the author typed mid-flight or if item ordering
// hasn't converged yet, so both settle without a manual nudge.
// ---------------------------------------------------------------------------

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	/** Server diagnostic, surfaced verbatim on `error`. */
	message?: string;
	code?: string;
}

/** A greeting in the editable draft. No `id` until the server mints one on the
 *  first successful save that includes it. `order` is the array index, so it
 *  isn't tracked here. */
export interface DraftGreeting {
	id?: Snowflake;
	text: string;
}

/** An example dialogue in the draft. Same id/ordering story as DraftGreeting. */
export interface DraftExample {
	id?: Snowflake;
	content: string;
}

/** The editable, flattened form of the active character. Metadata and
 *  definition scalars are hoisted to the top level so the form binds directly;
 *  server-managed fields (timestamps, version id) stay on the baseline. */
export interface CharacterDraft {
	id: Snowflake;
	// metadata (editable subset)
	display_name: string; // '' == none
	alt_names: string[];
	tags: string[];
	creator: string;
	creator_notes: string;
	card_version: string;
	// definition
	name: string;
	description: string;
	personality: string;
	scenario: string;
	system_prompt: string;
	post_history_instructions: string;
	greetings: DraftGreeting[];
	examples: DraftExample[];
}

class CharacterStore {
	private _api = $state<ApiClient>();
	metadataList = $state<Array<CharacterSummary>>([]);

  activeCharacter = $state<Character | null>(null);
	
	get #api() {
		if (!this._api) this._api = useApi(); // accessed once during page init
		return this._api;
	}

	/** Last server-confirmed character — the diff reference and the source of
	 *  display-only fields (timestamps, current_version_id). */
	#baseline = $state<Character | null>(null);

	/** The editable proxy the form binds to. */
	private _draft = $state<CharacterDraft | null>(null);

	/** Object-URL for the avatar blob, owned by this store (revoked on close). */
	avatarUrl = $state<string | null>(null);

	/** Whole-character save state. */
	saveState = $state<SaveState>({ status: 'idle' });

	/** Version history (newest first), refreshed on load and whenever a save
	 *  opens a new version. */
	versions = $state<CharacterVersionInfo[]>([]);

	/** True when the most recent user-initiated save opened a NEW backend version
	 *  rather than updating the current one. The backend groups edits within a
	 *  session window; this lights up only on the save that starts a fresh one. */
	lastSaveCreatedVersion = $state(false);

	// Save concurrency: one PUT in flight at a time; coalesce the rest.
	#saving = false;
	#resaveQueued = false;

	/* --- accessors -------------------------------------------------------- */

	get editingCharacter(): CharacterDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	/** Lorebook attached to the character, if any. Read-only — there is no edit
	 *  op for it, so the editor surfaces it but cannot change it here. */
	get lorebookId(): Snowflake | undefined {
		return this.#baseline?.lorebook_id;
	}
	get hasUnsavedChanges(): boolean {
		return this.#diffEdits().length > 0;
	}
	/** The version edits are currently landing in. */
	get currentVersionId(): Snowflake | null {
		return this.#baseline?.current_version_id ?? null;
	}

	/* --- loading ---------------------------------------------------------- */

	async loadList() {
		this.metadataList = await this.#api.characters.list();
  }

  async setActive(charId: Snowflake) {
    this.activeCharacter = await this.#api.characters.get(charId);
  }

	/* --- editing session -------------------------------------------------- */

	/** Open `id` for editing: fetch the full character, build the draft, and
	 *  load the avatar in the background (its failure must not block editing). */
	async startEditing(id: Snowflake) {
		this.#resetSession();
		const base = await this.#api.characters.get(id);
		this.#baseline = base;
		this._draft = buildDraft(base);
		void this.#loadAvatar(id);
		void this.loadVersions();
	}

	stopEditing() {
		this.#resetSession();
	}

	/** Throw away uncommitted draft changes, back to the last confirmed state. */
	discardChanges() {
		if (this.#baseline) this._draft = buildDraft(this.#baseline);
		this.saveState = { status: 'idle' };
	}

	#resetSession() {
		this._draft = null;
		this.#baseline = null;
		this.saveState = { status: 'idle' };
		this.#resaveQueued = false;
		this.versions = [];
		this.lastSaveCreatedVersion = false;
		if (this.avatarUrl) {
			URL.revokeObjectURL(this.avatarUrl);
			this.avatarUrl = null;
		}
	}

	/** Fetch the version history (newest first). Best-effort; a failure leaves
	 *  the existing list in place. */
	async loadVersions() {
		const id = this.editingId;
		if (!id) return;
		try {
			const list = await this.#api.characters.versions.list(id);
			if (this.editingId !== id) return; // session moved on while fetching
			this.versions = [...list].sort((a, b) => b.version_num - a.version_num);
		} catch {
			// leave the current list; the History view just won't refresh
		}
	}

	async #loadAvatar(id: Snowflake) {
		try {
			const blob = await this.#api.characters.avatar(id);
			if (this.editingId !== id) return; // session moved on while fetching
			this.avatarUrl = URL.createObjectURL(blob);
		} catch {
			// no avatar / fetch failed — leave it null, the editor shows a placeholder
		}
	}

	/* --- draft list mutations (local until next save) --------------------- */

	addGreeting() {
		this._draft?.greetings.push({ text: '' });
	}
	removeGreeting(index: number) {
		this._draft?.greetings.splice(index, 1);
	}
	moveGreeting(index: number, dir: -1 | 1) {
		swap(this._draft?.greetings, index, index + dir);
	}

	addExample() {
		this._draft?.examples.push({ content: '' });
	}
	removeExample(index: number) {
		this._draft?.examples.splice(index, 1);
	}
	moveExample(index: number, dir: -1 | 1) {
		swap(this._draft?.examples, index, index + dir);
	}

	/* --- saving ----------------------------------------------------------- */

	#diffEdits(): CharacterEdit[] {
		if (!this.#baseline || !this._draft) return [];
		return diffCharacter(this.#baseline, this._draft).edits;
	}

	/** Diff the draft against the baseline and persist the resulting op list.
	 *  Debounce the call site; this guards against overlapping PUTs itself.
	 *  `internal` marks the one follow-up save that converges new-item ordering —
	 *  it must not move the version marker, which reflects user-initiated saves. */
	async save(internal = false) {
		const draft = this._draft;
		const base = this.#baseline;
		if (!draft || !base) return;

		if (this.#saving) {
			this.#resaveQueued = true;
			return;
		}

		const { edits, addRefs } = diffCharacter(base, draft);
		if (edits.length === 0) {
			if (this.saveState.status === 'saving') this.saveState = { status: 'saved' };
			return;
		}

		const prevVersionId = base.current_version_id;
		const hadAdds = addRefs.greetings.length > 0 || addRefs.examples.length > 0;

		this.#saving = true;
		this.saveState = { status: 'saving' };
		let succeeded = false;
		try {
			const saved = await this.#api.characters.update(draft.id, edits);
			reconcileNewIds(saved, addRefs);
			// A changed current_version_id means the backend opened a new version
			// for this save rather than updating the active one.
			const createdVersion = saved.current_version_id !== prevVersionId;
			this.#baseline = saved;
			this.saveState = { status: 'saved' };
			if (!internal) this.lastSaveCreatedVersion = createdVersion;
			if (createdVersion) void this.loadVersions();
			succeeded = true;
		} catch (e) {
			// Preceding ops may have landed (see the API contract on `update`), so
			// re-base from the server before reporting. Reconcile ids too: if an
			// add applied before the failing op, the draft item needs its id or the
			// next diff would re-add it.
			try {
				const fresh = await this.#api.characters.get(draft.id);
				reconcileNewIds(fresh, addRefs);
				this.#baseline = fresh;
			} catch {
				// refetch failed — keep the stale baseline; the author can retry
			}
			this.saveState = { status: 'error', ...extractError(e) };
		} finally {
			this.#saving = false;
			if (this.#resaveQueued) {
				// The author edited while the save was in flight — coalesced retry.
				this.#resaveQueued = false;
				void this.save();
			} else if (succeeded && hadAdds && this.hasUnsavedChanges) {
				// New items just received ids. A reorder placing them among existing
				// items couldn't be expressed before (no ids), so emit it now. This
				// follow-up carries no adds, so it cannot chain — bounded to one pass.
				void this.save(true);
			}
		}
	}
}

/* --- draft construction ------------------------------------------------- */

function buildDraft(base: Character): CharacterDraft {
	return {
		id: base.id,
		display_name: base.metadata.display_name ?? '',
		alt_names: [...base.metadata.alt_names],
		tags: [...base.metadata.tags],
		creator: base.metadata.creator,
		creator_notes: base.metadata.creator_notes,
		card_version: base.metadata.card_version,
		name: base.definition.name,
		description: base.definition.description,
		personality: base.definition.personality,
		scenario: base.definition.scenario,
		system_prompt: base.definition.system_prompt,
		post_history_instructions: base.definition.post_history_instructions,
		greetings: orderedGreetings(base).map((g) => ({ id: g.id, text: g.text })),
		examples: orderedExamples(base).map((e) => ({ id: e.id, content: e.content }))
	};
}

const orderedGreetings = (c: Character) =>
	[...c.definition.greetings].sort((a, b) => a.order - b.order);
const orderedExamples = (c: Character) =>
	[...c.definition.example_messages].sort((a, b) => a.order - b.order);

/* --- diffing ------------------------------------------------------------ */

interface AddRefs {
	greetings: DraftGreeting[];
	examples: DraftExample[];
}

/** Produce the minimal op list that turns `base` into `draft`, plus references
 *  to the draft items that generated `add_*` ops (in emit order) so the caller
 *  can patch their server-minted ids back on once the save returns. */
function diffCharacter(
	base: Character,
	draft: CharacterDraft
): { edits: CharacterEdit[]; addRefs: AddRefs } {
	const edits: CharacterEdit[] = [];
	const addRefs: AddRefs = { greetings: [], examples: [] };

	// definition scalars (merge-patch: only changed fields)
	const def: EditDefinition = {};
	if (draft.name !== base.definition.name) def.name = draft.name;
	if (draft.description !== base.definition.description) def.description = draft.description;
	if (draft.personality !== base.definition.personality) def.personality = draft.personality;
	if (draft.scenario !== base.definition.scenario) def.scenario = draft.scenario;
	if (draft.system_prompt !== base.definition.system_prompt)
		def.system_prompt = draft.system_prompt;
	if (draft.post_history_instructions !== base.definition.post_history_instructions)
		def.post_history_instructions = draft.post_history_instructions;
	if (Object.keys(def).length > 0) edits.push({ op: 'definition', ...def });

	// metadata scalars
	const meta: EditMetadata = {};
	if (draft.creator !== base.metadata.creator) meta.creator = draft.creator;
	if (draft.creator_notes !== base.metadata.creator_notes) meta.creator_notes = draft.creator_notes;
	if (draft.card_version !== base.metadata.card_version) meta.card_version = draft.card_version;
	// display_name is three-state: '' means "none" (=> null to clear), else set.
	// Normalize both sides to a single "none" value so a cleared name round-trips
	// cleanly whether the server echoes it as null, '', undefined, or absent —
	// otherwise this op re-emits on every pass and autosave never settles.
	const draftDn = draft.display_name === '' ? undefined : draft.display_name;
	const baseDn = base.metadata.display_name || undefined;
	if (draftDn !== baseDn) meta.display_name = draftDn ?? null;
	if (Object.keys(meta).length > 0) edits.push({ op: 'metadata', ...meta });

	// whole-list replaces
	if (!arrayEq(draft.tags, base.metadata.tags))
		edits.push({ op: 'set_tags', tags: [...draft.tags] });
	if (!arrayEq(draft.alt_names, base.metadata.alt_names))
		edits.push({ op: 'set_alt_names', alt_names: [...draft.alt_names] });

	// greetings (item-level)
	diffList(
		orderedGreetings(base).map((g) => ({ id: g.id, value: g.text })),
		draft.greetings.map((g) => ({ ref: g, id: g.id, value: g.text })),
		{
			remove: (id) => edits.push({ op: 'remove_greeting', id }),
			update: (id, value) => edits.push({ op: 'update_greeting', id, text: value }),
			add: (ref, value) => {
				edits.push({ op: 'add_greeting', text: value });
				addRefs.greetings.push(ref as DraftGreeting);
			},
			reorder: (ids) => edits.push({ op: 'reorder_greetings', ids })
		}
	);

	// example dialogues (item-level)
	diffList(
		orderedExamples(base).map((e) => ({ id: e.id, value: e.content })),
		draft.examples.map((e) => ({ ref: e, id: e.id, value: e.content })),
		{
			remove: (id) => edits.push({ op: 'remove_example', id }),
			update: (id, value) => edits.push({ op: 'update_example', id, content: value }),
			add: (ref, value) => {
				edits.push({ op: 'add_example', content: value });
				addRefs.examples.push(ref as DraftExample);
			},
			reorder: (ids) => edits.push({ op: 'reorder_examples', ids })
		}
	);

	return { edits, addRefs };
}

interface BaseItem {
	id: Snowflake;
	value: string;
}
interface DraftItem {
	ref: DraftGreeting | DraftExample;
	id?: Snowflake;
	value: string;
}
interface ListOps {
	remove: (id: Snowflake) => void;
	update: (id: Snowflake, value: string) => void;
	add: (ref: DraftGreeting | DraftExample, value: string) => void;
	reorder: (ids: Snowflake[]) => void;
}

/** Shared item-list diff for greetings and examples. Removes, then updates,
 *  then a reorder over surviving ids if their relative order changed, then
 *  adds (new items have no id yet, so they append server-side and are placed
 *  by a follow-up reorder on the next save if needed). */
function diffList(baseList: BaseItem[], draftList: DraftItem[], ops: ListOps) {
	const baseById = new Map(baseList.map((b) => [b.id, b.value]));
	const draftIds = new Set(draftList.filter((d) => d.id).map((d) => d.id!));

	for (const b of baseList) {
		if (!draftIds.has(b.id)) ops.remove(b.id);
	}
	for (const d of draftList) {
		if (d.id) {
			if (baseById.has(d.id) && baseById.get(d.id) !== d.value) ops.update(d.id, d.value);
		} else {
			ops.add(d.ref, d.value);
		}
	}

	const draftOrder = draftList.filter((d) => d.id).map((d) => d.id!);
	const baseOrder = baseList.filter((b) => draftIds.has(b.id)).map((b) => b.id);
	if (!arrayEq(draftOrder, baseOrder)) ops.reorder(draftOrder);
}

/** Patch server-minted ids onto the draft items that produced `add_*` ops.
 *  The freshly created server items are those whose ids the draft doesn't yet
 *  claim; mapped in order to the add refs (both follow creation order). */
function reconcileNewIds(saved: Character, addRefs: AddRefs) {
	assignIds(
		orderedGreetings(saved).map((g) => g.id),
		addRefs.greetings
	);
	assignIds(
		orderedExamples(saved).map((e) => e.id),
		addRefs.examples
	);
}

function assignIds(serverIds: Snowflake[], refs: Array<{ id?: Snowflake }>) {
	const claimed = new Set(refs.filter((r) => r.id).map((r) => r.id!));
	const fresh = serverIds.filter((id) => !claimed.has(id));
	const pending = refs.filter((r) => !r.id);
	pending.forEach((r, i) => {
		if (i < fresh.length) r.id = fresh[i];
	});
}

/* --- small helpers ------------------------------------------------------ */

function arrayEq(a: string[], b: string[]): boolean {
	return a.length === b.length && a.every((v, i) => v === b[i]);
}

function swap<T>(arr: T[] | undefined, i: number, j: number) {
	if (!arr || i < 0 || j < 0 || i >= arr.length || j >= arr.length) return;
	[arr[i], arr[j]] = [arr[j], arr[i]];
}

/** Unwrap an http rejection into the server's `{ message, code }`. */
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

export const characterStore = new CharacterStore();
