import { useApi } from '#lib/api/client.js';
import type { ServerConfig, ConfigKey } from '#lib/types/config.js';
import type { ErrorCode, ErrorDetails } from '#lib/types/error.js';
import { Permission } from '#lib/types/permissions.js';

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

	/** The user's effective permissions, Administrator already expanded. */
	permissions = $state(0n);
	/** The user can manage roles positioned below this. */
	highestPosition = $state(0);

	can(flag: bigint): boolean {
		return (this.permissions & flag) === flag;
	}

	get canEditServer(): boolean {
		return this.can(Permission.MANAGE_SERVER);
	}

	/** Server settings are shown to anyone who can change some part of them. */
	get canSeeServer(): boolean {
		return (
			this.canEditServer || this.can(Permission.MANAGE_ROLES) || this.can(Permission.MANAGE_USERS)
		);
	}

	async load() {
		this.config = await this.#api.config.getAll();
		this.loaded = true;
	}

	async loadPermissions() {
		const mine = await this.#api.users.permissions();
		this.permissions = BigInt(mine.permissions);
		this.highestPosition = mine.highest_position;
	}

	// Sends the full section object
	async patch<K extends ConfigKey>(key: K, value: ServerConfig[K]): Promise<void> {
		const next = await this.#api.config.patch(key, value);
		if (this.config) this.config[key] = next;
	}
}

export const configStore = new ConfigStore();
