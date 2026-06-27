import { useApi, type ApiClient } from '$lib/api/client.js';
import type { ServerConfig, ConfigKey } from '$lib/types/config.js';

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error';
export interface SaveState {
	status: SaveStatus;
	code?: string;
	message?: string;
}

export enum SettingsSection {
	Hosting,
	Storage,
	Sessions,
	Users,
	Characters,
	Tokenizers,
	Tasks
}

class ConfigStore {
	private _api = $state<ApiClient>();

	get #api() {
		if (!this._api) this._api = useApi();
		return this._api;
	}

	config = $state<ServerConfig>();
	loaded = $state(false);

	// Hardcoded for now.
	// TODO: Implement real permissions. Depends on backend work
	canEditServer = $state(true);

	section = $state(SettingsSection.Hosting);
	setSection(section: SettingsSection) {
		this.section = section;
	}

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
