import { useApi, type ApiClient } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { Tokenizer, NetworkCreate, ImportQuery } from '$lib/types/tokenizer.js';

class TokenizerStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi();
		return this._api;
	}

	tokenizers = $state<Tokenizer[]>([]);
	activeId = $state<Snowflake | null>(null);
	loaded = $state(false);

	async load() {
		const [list, active] = await Promise.all([
			this.#api.tokenizers.list(),
			this.#api.tokenizers.active.get()
		]);
		this.tokenizers = list;
		this.activeId = active;
		this.loaded = true;
	}

	async setActive(id: Snowflake) {
		await this.#api.tokenizers.active.set({ id: id });
		this.activeId = id;
	}

	async clearActive() {
		await this.#api.tokenizers.active.unset();
		this.activeId = null;
	}

	async remove(id: Snowflake) {
		await this.#api.tokenizers.delete(id);
		this.tokenizers = this.tokenizers.filter((t) => t.id !== id);
		if (this.activeId === id) this.activeId = null;
	}

	async importFile(query: ImportQuery, blob: Blob) {
		await this.#api.tokenizers.imports.file(blob);
		await this.load();
	}

	async importNetwork(create: NetworkCreate) {
		await this.#api.tokenizers.imports.network(create);
		await this.load();
	}
}

export const tokenizerStore = new TokenizerStore();
