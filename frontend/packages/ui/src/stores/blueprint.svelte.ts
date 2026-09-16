import { useApi } from '$lib/api/client.js';
import type { Blueprint, BlueprintSummary, InputSpec } from '$lib/types/blueprint.js';
import type { Snowflake } from '$lib/types/core.js';
import type { Realm } from '$lib/types/realm.js';
import { showApiError } from './errorActions.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';

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

	async open(id: Snowflake) {
		this.current = await this.#api.blueprints.get(id);
		await this.loadInputs();
	}

	close() {
		this.current = null;
		this.inputs = [];
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

	/** Write the document. */
	async save(name: string, source: string): Promise<boolean> {
		const open = this.current;
		if (!open) return false;
		this.saveStatus = 'saving';
		this.saveError = null;
		try {
			this.current = await this.#api.blueprints.update(open.id, { name, source });
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
