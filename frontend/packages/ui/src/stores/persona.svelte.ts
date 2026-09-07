import { extractError } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { PersonaOverride, UpdatePersona, UserPersona } from '$lib/types/persona.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

export interface SaveState {
	status: SaveStatus;
	message?: string;
	code?: string;
}

/** The editable form of the active persona. Empty string means "no value". */
export interface PersonaDraft {
	id: Snowflake;
	name: string;
	display_name: string; // '' == none
	pseudonym: string; // '' == none
	description: string; // '' == none
}

class PersonaStore {
	/** Listing-only metadata */
	loadedPersonas = $state<Array<UserPersona>>();

	/** The user's currently active persona, or null. App-wide, not tied to editing. */
	activePersonaId = $state<Snowflake | null>(null);

	/** Character-level bindings that win over `activePersonaId`. */
	overrides = $state<Array<PersonaOverride>>([]);

	get #api() {
		return useApi();
	}

	#baseline = $state<UserPersona | null>(null);
	/** The editable proxy */
	private _draft = $state<PersonaDraft | null>(null);

	saveState = $state<SaveState>({ status: 'idle' });

	#saving = false;
	#resaveQueued = false;

	get editingPersona(): PersonaDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}
	get hasUnsavedChanges(): boolean {
		if (!this.#baseline || !this._draft) return false;
		return Object.keys(diffPersona(this.#baseline, this._draft)).length > 0;
	}

	async loadPersonas() {
		const personas = await this.#api?.personas.list();

		if (personas) this.loadedPersonas = personas;
	}

	/** Fetch which persona is currently active. */
	async loadActive() {
		this.activePersonaId = await this.#api.personas.active.get();
	}

	/** Mark a persona as active. Optimistic, reverting if the request fails. */
	async setActive(id: Snowflake) {
		if (this.activePersonaId === id) return;
		const previous = this.activePersonaId;
		this.activePersonaId = id;
		try {
			await this.#api.personas.active.set({ id });
		} catch (e) {
			this.activePersonaId = previous;
			throw e;
		}
	}

	async create(): Promise<UserPersona> {
		const persona = await this.#api.personas.create();
		this.loadedPersonas = [...(this.loadedPersonas ?? []), persona];
		return persona;
	}

	async loadOverrides() {
		this.overrides = await this.#api.personas.overrides.list();
	}

	overridesFor(personaId: Snowflake): PersonaOverride[] {
		return this.overrides.filter((o) => o.persona_id === personaId);
	}

	overrideFor(characterId: Snowflake): PersonaOverride | undefined {
		return this.overrides.find((o) => o.character_id === characterId);
	}

	async setOverride(characterId: Snowflake, personaId: Snowflake) {
		const saved = await this.#api.personas.overrides.set(characterId, { persona_id: personaId });
		const i = this.overrides.findIndex((o) => o.character_id === characterId);
		if (i === -1) this.overrides = [...this.overrides, saved];
		else this.overrides[i] = saved;
	}

	async clearOverride(characterId: Snowflake) {
		const previous = this.overrides;
		this.overrides = this.overrides.filter((o) => o.character_id !== characterId);
		try {
			await this.#api.personas.overrides.clear(characterId);
		} catch (e) {
			this.overrides = previous;
			throw e;
		}
	}

	async startEditing(id: Snowflake) {
		this.#resetSession();
		if (!this.loadedPersonas) await this.loadPersonas();
		const found = this.loadedPersonas?.find((p) => p.id === id) ?? null;
		if (!found) {
			this.saveState = { status: 'error', message: 'Persona not found' };
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

	async setAvatar(image: Blob) {
		const draft = this._draft;
		if (!draft) return;
		const id = draft.id;

		const { avatar_uri } = await this.#api.personas.setAvatar(id, image);
		if (this.editingId !== id) return; // session moved on mid-upload

		const listed = this.loadedPersonas?.find((p) => p.id === id);
		if (listed) listed.avatar_uri = avatar_uri;
		if (this.#baseline) this.#baseline.avatar_uri = avatar_uri;
	}

	#resetSession() {
		this._draft = null;
		this.#baseline = null;
		this.saveState = { status: 'idle' };
		this.#resaveQueued = false;
	}

	async save() {
		const draft = this._draft;
		const base = this.#baseline;
		if (!draft || !base) return;

		if (this.#saving) {
			this.#resaveQueued = true;
			return;
		}

		const patch = diffPersona(base, draft);
		if (Object.keys(patch).length === 0) {
			if (this.saveState.status === 'saving') this.saveState = { status: 'saved' };
			return;
		}

		this.#saving = true;
		this.saveState = { status: 'saving' };
		try {
			const saved = await this.#api.personas.update(draft.id, patch);
			if (this.editingId !== draft.id) return; // session moved on mid-save
			this.#baseline = saved;
			this.#syncListEntry(saved);
			this.saveState = { status: 'saved' };
		} catch (e) {
			this.saveState = { status: 'error', ...extractError(e) };
		} finally {
			this.#saving = false;
			if (this.#resaveQueued) {
				// The author edited while the save was in flight.
				this.#resaveQueued = false;
				void this.save();
			}
		}
	}

	/** Keep the list in sync so the sidebar reflects the saved name. */
	#syncListEntry(p: UserPersona) {
		const list = this.loadedPersonas;
		if (!list) return;
		const i = list.findIndex((x) => x.id === p.id);
		if (i !== -1) list[i] = p;
	}
}

function buildDraft(p: UserPersona): PersonaDraft {
	return {
		id: p.id,
		name: p.name,
		display_name: p.display_name ?? '',
		pseudonym: p.pseudonym ?? '',
		description: p.description ?? ''
	};
}

/**
 * Build a merge-patch. A field absent from the result means "no change"
 * (the double-null case); an explicit `null` clears the field server-side.
 */
function diffPersona(base: UserPersona, draft: PersonaDraft): UpdatePersona {
	const patch: UpdatePersona = {};
	if (draft.name !== base.name) patch.name = draft.name;
	setNullable(patch, 'display_name', draft.display_name, base.display_name);
	setNullable(patch, 'pseudonym', draft.pseudonym, base.pseudonym);
	setNullable(patch, 'description', draft.description, base.description);
	return patch;
}

function setNullable(
	patch: UpdatePersona,
	key: 'display_name' | 'pseudonym' | 'description',
	draftVal: string,
	baseVal: string | null
) {
	const next = draftVal === '' ? null : draftVal; // '' in the form == cleared
	const prev = baseVal ?? null;
	if (next !== prev) patch[key] = next;
}

export const personaStore = new PersonaStore();
