import { extractError } from '#lib/types/error.js';
import { useApi } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';
import type {
	AttachmentKind,
	Character,
	ExportProfileInfo,
	ImportOptions,
	CharacterEdit,
	CharacterSummary,
	CharacterVersionInfo,
	EditDefinition,
	EditMetadata,
	ImportResult
} from '#lib/types/character.js';

const MAX_IMPORT_BATCH_BYTES = 24 * 1024 * 1024;

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: string;
}

export interface DraftGreeting {
	id?: Snowflake;
	text: string;
}

export interface DraftExample {
	id?: Snowflake;
	content: string;
}

/** The editable, flattened form of the active character. */
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
	template_injection: string;
	greetings: DraftGreeting[];
	examples: DraftExample[];
	lorebook_ids: Snowflake[];
	tool_ids: Snowflake[];
	widget_ids: Snowflake[];
}

const ATTACHMENT_FIELD = {
	lorebook: 'lorebook_ids',
	tool: 'tool_ids',
	widget: 'widget_ids'
} as const satisfies Record<AttachmentKind, keyof CharacterDraft>;

class CharacterStore {
	metadataList = $state<Array<CharacterSummary>>([]);

	activeCharacter = $state<Character | null>(null);

	get #api() {
		return useApi();
	}

	#baseline = $state<Character | null>(null);

	/** The editable proxy */
	private _draft = $state<CharacterDraft | null>(null);

	avatarUrl = $state<string | null>(null);

	saveState = $state<SaveState>({ status: 'idle' });

	/** Version history */
	versions = $state<CharacterVersionInfo[]>([]);

	lastSaveCreatedVersion = $state(false);

	#saving = false;
	#resaveQueued = false;

	get editingCharacter(): CharacterDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	/** Lorebook attached to the character */
	get hasUnsavedChanges(): boolean {
		return this.#diffEdits().length > 0;
	}
	/** The version edits are currently landing in. */
	get currentVersionId(): Snowflake | null {
		return this.#baseline?.current_version_id ?? null;
	}

	async loadList() {
		this.metadataList = await this.#api.characters.list();
	}

	/** Export profiles the server offers. Loaded once, on first use. */
	exportProfiles = $state<ExportProfileInfo[]>([]);
	#exportProfilesLoading: Promise<void> | null = null;

	loadExportProfiles(): Promise<void> {
		this.#exportProfilesLoading ??= this.#api.characters
			.exportProfiles()
			.then((profiles) => {
				this.exportProfiles = profiles;
			})
			.catch((e) => {
				this.#exportProfilesLoading = null; // try again next time
				throw e;
			});
		return this.#exportProfilesLoading;
	}

	async setActive(charId: Snowflake) {
		this.activeCharacter = await this.#api.characters.get(charId);
	}

	async create(name?: string): Promise<Snowflake> {
		const created = await this.#api.characters.create({ name });
		await this.loadList();
		return created.id;
	}

	/** Delete a character and its chats. Waits for the server before touching local state. */
	async delete(id: Snowflake) {
		await this.#api.characters.delete(id);
		if (this.metadataList) this.metadataList = this.metadataList.filter((c) => c.id !== id);
		if (this.editingId === id) this.stopEditing();
		if (this.activeCharacter?.id === id) this.activeCharacter = null;
	}

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

	/** Fetch the version history */
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
			// no avatar / fetch failed.
		}
	}

	async changeImage(image: Blob) {
		const draft = this._draft;
		if (!draft) return;
		const id = draft.id;

		const { avatar_uri } = await this.#api.characters.updateAvatar(id, image);
		if (this.editingId !== id) return; // session moved on mid-upload

		// Swap the preview to the local bytes — the image is already in hand,
		// so there's no reason to refetch it from the server.
		const previous = this.avatarUrl;
		this.avatarUrl = URL.createObjectURL(image);
		if (previous) URL.revokeObjectURL(previous);

		if (this.#baseline) this.#baseline.metadata.avatar_uri = avatar_uri;

		if (this.activeCharacter?.id === id) this.activeCharacter.metadata.avatar_uri = avatar_uri;
		const listed = this.metadataList.find((c) => c.id === id);
		if (listed) listed.avatar_uri = avatar_uri;
	}

	/** Import cards in bulk. */
	async import(
		cards: Blob[],
		reloadList: boolean = false,
		options: ImportOptions = {}
	): Promise<ImportResult[]> {
		const results: ImportResult[] = [];
		for (const batch of batchBySize(cards, MAX_IMPORT_BATCH_BYTES)) {
			const response = await this.#api.characters.importBatch(batch, options);
			results.push(...response.results);
		}
		if (reloadList) await this.loadList();
		return results;
	}

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

	attached(kind: AttachmentKind): Snowflake[] {
		return this._draft?.[ATTACHMENT_FIELD[kind]] ?? [];
	}
	isAttached(kind: AttachmentKind, id: Snowflake): boolean {
		return this.attached(kind).includes(id);
	}
	toggleAttachment(kind: AttachmentKind, id: Snowflake) {
		const draft = this._draft;
		if (!draft) return;
		const list = draft[ATTACHMENT_FIELD[kind]];
		const at = list.indexOf(id);
		if (at === -1) list.push(id);
		else list.splice(at, 1);
	}
	moveAttachment(kind: AttachmentKind, index: number, dir: -1 | 1) {
		swap(this._draft?.[ATTACHMENT_FIELD[kind]], index, index + dir);
	}

	async duplicateAttachment(kind: AttachmentKind, id: Snowflake): Promise<Snowflake> {
		const charId = this.editingId;
		if (!charId) throw new Error('No character is being edited');

		while (this.#saving) await new Promise((r) => setTimeout(r, 50));
		if (this.hasUnsavedChanges) await this.save(true);

		const { character, new_id } = await this.#api.characters.duplicateAttachment(charId, kind, id);
		if (this.editingId !== charId) return new_id; // session moved on meanwhile

		this.#baseline = character;
		const list = this._draft?.[ATTACHMENT_FIELD[kind]];
		const at = list?.indexOf(id) ?? -1;
		if (list && at !== -1) list[at] = new_id;
		return new_id;
	}

	#diffEdits(): CharacterEdit[] {
		if (!this.#baseline || !this._draft) return [];
		return diffCharacter(this.#baseline, this._draft).edits;
	}

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
			// re-base from the server before reporting.
			try {
				const fresh = await this.#api.characters.get(draft.id);
				reconcileNewIds(fresh, addRefs);
				this.#baseline = fresh;
			} catch {
				// refetch failed
			}
			this.saveState = { status: 'error', ...extractError(e) };
		} finally {
			this.#saving = false;
			if (this.#resaveQueued) {
				// The author edited while the save was in flight
				this.#resaveQueued = false;
				void this.save();
			} else if (succeeded && hadAdds && this.hasUnsavedChanges) {
				void this.save(true);
			}
		}
	}
}

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
		template_injection: base.definition.template_injection ?? '',
		greetings: orderedGreetings(base).map((g) => ({ id: g.id, text: g.text })),
		examples: orderedExamples(base).map((e) => ({ id: e.id, content: e.content })),
		lorebook_ids: [...(base.lorebook_ids ?? [])],
		tool_ids: [...(base.tool_ids ?? [])],
		widget_ids: [...(base.widget_ids ?? [])]
	};
}

const orderedGreetings = (c: Character) =>
	[...c.definition.greetings].sort((a, b) => a.order - b.order);
const orderedExamples = (c: Character) =>
	[...c.definition.example_messages].sort((a, b) => a.order - b.order);

interface AddRefs {
	greetings: DraftGreeting[];
	examples: DraftExample[];
}

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
	if (draft.template_injection !== (base.definition.template_injection ?? ''))
		def.template_injection = draft.template_injection;
	if (Object.keys(def).length > 0) edits.push({ op: 'definition', ...def });

	// metadata scalars
	const meta: EditMetadata = {};
	if (draft.creator !== base.metadata.creator) meta.creator = draft.creator;
	if (draft.creator_notes !== base.metadata.creator_notes) meta.creator_notes = draft.creator_notes;
	if (draft.card_version !== base.metadata.card_version) meta.card_version = draft.card_version;
	// display_name is three-state. See field docs.
	const draftDn = draft.display_name === '' ? undefined : draft.display_name;
	const baseDn = base.metadata.display_name || undefined;
	if (draftDn !== baseDn) meta.display_name = draftDn ?? null;
	if (Object.keys(meta).length > 0) edits.push({ op: 'metadata', ...meta });

	// whole-list replaces
	if (!arrayEq(draft.tags, base.metadata.tags))
		edits.push({ op: 'set_tags', tags: [...draft.tags] });
	if (!arrayEq(draft.alt_names, base.metadata.alt_names))
		edits.push({ op: 'set_alt_names', alt_names: [...draft.alt_names] });
	if (!arrayEq(draft.widget_ids, base.widget_ids ?? []))
		edits.push({ op: 'set_widgets', widget_ids: [...draft.widget_ids] });
	if (!arrayEq(draft.lorebook_ids, base.lorebook_ids ?? []))
		edits.push({ op: 'set_lorebooks', lorebook_ids: [...draft.lorebook_ids] });
	if (!arrayEq(draft.tool_ids, base.tool_ids ?? []))
		edits.push({ op: 'set_tools', tool_ids: [...draft.tool_ids] });

	// greetings
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

	// example dialogues
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

function arrayEq(a: string[], b: string[]): boolean {
	return a.length === b.length && a.every((v, i) => v === b[i]);
}

function swap<T>(arr: T[] | undefined, i: number, j: number) {
	if (!arr || i < 0 || j < 0 || i >= arr.length || j >= arr.length) return;
	[arr[i], arr[j]] = [arr[j], arr[i]];
}

/**
 * Group cards into batches that each stay under `maxBytes`.
 */
function batchBySize(cards: Blob[], maxBytes: number): Blob[][] {
	const batches: Blob[][] = [];
	let batch: Blob[] = [];
	let size = 0;

	for (const card of cards) {
		if (batch.length > 0 && size + card.size > maxBytes) {
			batches.push(batch);
			batch = [];
			size = 0;
		}
		batch.push(card);
		size += card.size;
	}
	if (batch.length > 0) batches.push(batch);

	return batches;
}

export const characterStore = new CharacterStore();
