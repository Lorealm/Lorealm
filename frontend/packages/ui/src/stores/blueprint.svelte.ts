import { useApi } from '$lib/api/client.js';
import type {
	Blueprint,
	BlueprintSummary,
	InputSpec,
	OutlineResult
} from '$lib/types/blueprint.js';
import type { Snowflake } from '$lib/types/core.js';
import type { Realm } from '$lib/types/realm.js';
import { showApiError } from './errorActions.js';

export type SaveStatus = 'idle' | 'dirty' | 'saving' | 'saved' | 'error' | 'blocked';

const SAVE_DEBOUNCE = 800;

/**
 * The blueprints the user owns, and whichever one the editor has open.
 */
class BlueprintStore {
	get #api() {
		return useApi();
	}

	summaries = $state<BlueprintSummary[]>([]);
	current = $state<Blueprint | null>(null);
	inputs = $state<InputSpec[]>([]);

	saveStatus = $state<SaveStatus>('idle');
	/** What the server refused, when it refused. */
	saveError = $state<string | null>(null);

	async loadList() {
		try {
			this.summaries = await this.#api.blueprints.list();
		} catch (e) {
			showApiError(e, { title: 'Failed to load blueprints' });
		}
	}

	/** Why the open blueprint could not be loaded into the cast pane, if so. */
	refusal = $state<string | null>(null);

	async open(id: Snowflake): Promise<OutlineResult | null> {
		this.current = await this.#api.blueprints.get(id);
		await this.loadInputs();
		try {
			const result = await this.#api.blueprints.outline(id);
			this.refusal = result.status === 'refused' ? result.reason.message : null;
			return result;
		} catch (e) {
			showApiError(e, { title: 'Failed to read the blueprint' });
			this.refusal = null;
			return null;
		}
	}

	close() {
		this.current = null;
		this.inputs = [];
		this.refusal = null;
		this.#clearTimer();
		this.#saved = null;
		this.#pending = null;
		this.saveStatus = 'idle';
		this.saveError = null;
	}

	async create(name: string, source: string): Promise<Blueprint | null> {
		try {
			const made = await this.#api.blueprints.create({ name, source });
			this.current = made;
			await this.loadList();
			return made;
		} catch (e) {
			showApiError(e, { title: 'Failed to create blueprint' });
			return null;
		}
	}

	/** The document as last written. */
	#saved: { name: string; source: string; empty: boolean } | null = null;
	#pending: { name: string; source: string; empty: boolean } | null = null;
	#timer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Record what the editor holds right now as already-saved.
	 */
	beginEditing(name: string, source: string, empty: boolean) {
		this.#saved = { name, source, empty };
	}

	/** Take the editor's current output and write it if it differs. */
	queueSave(name: string, source: string, empty: boolean) {
		if (!this.current || this.#saved === null) return;
		if (this.#saved.name === name && this.#saved.source === source) return;

		if (empty && !this.#saved.empty) {
			this.#clearTimer();
			this.#pending = null;
			this.saveStatus = 'blocked';
			return;
		}

		this.saveStatus = 'dirty';
		this.#pending = { name, source, empty };
		if (this.#timer) clearTimeout(this.#timer);
		this.#timer = setTimeout(() => void this.save(name, source, empty), SAVE_DEBOUNCE);
	}

	async flush(): Promise<void> {
		const pending = this.#pending;
		this.#clearTimer();
		if (pending) await this.save(pending.name, pending.source, pending.empty);
	}

	#clearTimer() {
		if (this.#timer) clearTimeout(this.#timer);
		this.#timer = null;
	}

	/** Write the document. */
	async save(name: string, source: string, empty: boolean): Promise<boolean> {
		const open = this.current;
		if (!open) return false;
		this.#clearTimer();
		this.saveStatus = 'saving';
		this.saveError = null;
		try {
			this.current = await this.#api.blueprints.update(open.id, { name, source });
			this.#saved = { name, source, empty };
			this.#pending = null;
			await this.loadInputs();
			await this.loadList();
			this.saveStatus = 'saved';
			return true;
		} catch (e) {
			this.saveStatus = 'error';
			this.saveError = e instanceof Error ? e.message : String(e);
			return false;
		}
	}

	async loadInputs() {
		if (!this.current) return;
		try {
			this.inputs = await this.#api.blueprints.inputs(this.current.id);
		} catch {
			// A document that will not parse has no declarations to report;
			this.inputs = [];
		}
	}

	async delete(id: Snowflake) {
		try {
			await this.#api.blueprints.delete(id);
			if (this.current?.id === id) this.close();
			await this.loadList();
		} catch (e) {
			showApiError(e, { title: 'Failed to delete blueprint' });
		}
	}

	/** Build a realm from a blueprint */
	async expand(id: Snowflake, inputs: Record<string, unknown>): Promise<Realm | null> {
		try {
			return await this.#api.blueprints.expand(id, inputs);
		} catch (e) {
			showApiError(e, { title: 'Failed to build a realm' });
			return null;
		}
	}
}

export const blueprintStore = new BlueprintStore();
