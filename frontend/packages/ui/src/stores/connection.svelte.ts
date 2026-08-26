import { extractError } from '$lib/types/error.js';
import { useApi } from '$lib/api/client.js';
import type { JsonValue, Snowflake } from '$lib/types/core.js';
import type {
	ConnectionProfile,
	CreateSecretRequest,
	DeleteSecretResponse,
	ProfileRequest,
	SecretMetadata
} from '$lib/types/connection.js';
import type { SaveState } from './lorebook.svelte.js';
import { TOKEN_BUDGET, type ProviderModel } from '$lib/types/schema.js';
import { SvelteMap } from 'svelte/reactivity';

export interface ProfileDraft {
	id: Snowflake;
	name: string;
	provider: string;
	model: string;
	token_budget: number;
	secret_id: Snowflake | null;
	common_params: Record<string, JsonValue>;
	provider_params: Record<string, JsonValue>;
}

class ConnectionStore {
	get #api() {
		return useApi();
	}

	profiles = $state<ConnectionProfile[]>();
	secrets = $state<SecretMetadata[]>();
	modelsByProfile = new SvelteMap<Snowflake, ProviderModel[]>();
	activeProfile = $state<Snowflake | null>();

	private _draft = $state<ProfileDraft | null>(null);
	saveState = $state<SaveState>({ status: 'idle' });

	get editingProfile(): ProfileDraft | null {
		return this._draft;
	}
	get editingId(): Snowflake | null {
		return this._draft?.id ?? null;
	}

	secretsFor(provider: string): SecretMetadata[] {
		return (this.secrets ?? []).filter((s) => s.provider === provider);
	}

	async loadProfiles() {
		this.profiles = await this.#api.connection.profiles.list();
	}

	async loadSecrets() {
		this.secrets = await this.#api.connection.secrets.list();
	}

	async loadModels(profileId: Snowflake) {
		if (this.modelsByProfile.has(profileId)) return;
		try {
			this.modelsByProfile.set(profileId, await this.#api.connection.profiles.models(profileId));
		} catch (e) {
			console.error(e);
			this.modelsByProfile.set(profileId, []);
		}
	}

	startEditing(id: Snowflake) {
		const base = this.profiles?.find((p) => p.id === id);
		if (!base) return;
		this.saveState = { status: 'idle' };
		this._draft = buildDraft(base);
	}

	stopEditing() {
		this._draft = null;
		this.saveState = { status: 'idle' };
	}

	discardChanges() {
		const draft = this._draft;
		if (!draft) return;
		const base = this.profiles?.find((p) => p.id === draft.id);
		if (base) this._draft = buildDraft(base);
		this.saveState = { status: 'idle' };
	}

	async saveProfile() {
		const draft = this._draft;
		if (!draft) return;
		this.saveState = { status: 'saving' };
		try {
			const req: ProfileRequest = {
				secret_id: draft.secret_id,
				name: draft.name,
				provider: draft.provider,
				model: draft.model,
				token_budget: draft.token_budget,
				common_params: $state.snapshot(draft.common_params) as unknown as JsonValue,
				provider_params: $state.snapshot(draft.provider_params) as unknown as JsonValue
			};
			const saved = await this.#api.connection.profiles.update(draft.id, req);
			const i = this.profiles?.findIndex((p) => p.id === saved.id) ?? -1;
			if (this.profiles && i !== -1) {
				if (endpointOf(this.profiles[i]) !== endpointOf(saved)) {
					this.modelsByProfile.delete(saved.id);
				}
				this.profiles[i] = saved;
			}
			this.saveState = { status: 'saved' };
		} catch (e) {
			this.saveState = { status: 'error', ...extractError(e) };
		}
	}

	async createProfile(name: string, provider: string): Promise<Snowflake | undefined> {
		const created = await this.#api.connection.profiles.create({
			secret_id: null,
			name,
			provider,
			model: '',
			token_budget: TOKEN_BUDGET.default,
			common_params: {},
			provider_params: {}
		});
		this.profiles?.push(created);
		return created.id;
	}

	async deleteProfile(id: Snowflake) {
		if (!this.profiles) return;
		const i = this.profiles.findIndex((p) => p.id === id);
		if (i === -1) return;
		const removed = this.profiles[i];
		this.profiles.splice(i, 1);
		if (this._draft?.id === id) this.stopEditing();

		try {
			await this.#api.connection.profiles.delete(id);
		} catch (e) {
			this.profiles.splice(i, 0, removed);
			throw e;
		}
	}

	async createSecret(req: CreateSecretRequest): Promise<SecretMetadata> {
		const created = await this.#api.connection.secrets.create(req);
		this.secrets?.push(created);
		return created;
	}

	async renameSecret(id: Snowflake, label: string) {
		const saved = await this.#api.connection.secrets.rename(id, { label });
		const i = this.secrets?.findIndex((s) => s.id === id) ?? -1;
		if (this.secrets && i !== -1) this.secrets[i] = saved;
	}

	async deleteSecret(id: Snowflake) {
		const res = (await this.#api.connection.secrets.delete(id)) as DeleteSecretResponse;
		this.secrets = this.secrets?.filter((s) => s.id !== id);
		for (const pid of res?.detached_profiles ?? []) {
			const p = this.profiles?.find((p) => p.id === pid);
			if (p) p.secret_id = null;
			if (this._draft?.id === pid) this._draft.secret_id = null;
		}
	}

	async getActiveProfile() {
		const res = await this.#api.connection.profiles.active.get();
		this.activeProfile = res;
	}

	async setActiveProfile(id: Snowflake) {
		await this.#api.connection.profiles.active.set({ id });
		this.activeProfile = id;
	}
}

function endpointOf(profile: ConnectionProfile): string {
	return JSON.stringify([profile.provider, profile.secret_id, profile.provider_params]);
}

function buildDraft(base: ConnectionProfile): ProfileDraft {
	return {
		id: base.id,
		name: base.name,
		provider: base.provider,
		model: base.model,
		token_budget: base.token_budget,
		secret_id: base.secret_id,
		common_params: asParams(base.common_params),
		provider_params: asParams(base.provider_params)
	};
}

function asParams(v: JsonValue): Record<string, JsonValue> {
	if (v && typeof v === 'object' && !Array.isArray(v)) {
		const snap = $state.snapshot(v) as unknown as Record<string, JsonValue>;
		const out: Record<string, JsonValue> = {};
		for (const [key, val] of Object.entries(snap)) {
			if (val !== null && val !== undefined) out[key] = val;
		}
		return structuredClone(out);
	}
	return {};
}

export const connectionStore = new ConnectionStore();
