import { useApi } from '$lib/api/client.js';
import type { ServerConfig, ConfigKey } from '$lib/types/config.js';
import type { ErrorCode, ErrorDetails } from '$lib/types/error.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';
export interface SaveState {
	status: SaveStatus;
	code?: ErrorCode;
	message?: string;
	details?: ErrorDetails;
}

class ConfigStore {
	get #api() {
		return useApi();
	}

	config = $state<ServerConfig>();
	loaded = $state(false);

	// Hardcoded for now.
	// TODO: Implement real permissions. Depends on backend work
	canEditServer = $state(true);

	async load() {
		this.config = await this.#api.config.getAll();
		this.loaded = true;
	}

	// Sends the full section object
	async patch<K extends ConfigKey>(key: K, value: ServerConfig[K]): Promise<void> {
		const next = await this.#api.config.patch(key, value);
		if (this.config) this.config[key] = next;
	}
}

export const configStore = new ConfigStore();
