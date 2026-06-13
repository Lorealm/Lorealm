import { useApi, type ApiClient } from '$lib/api/client.js';
import type { JsonValue, Snowflake } from '$lib/types/core.js';
import type {
	ConnectionProfile,
	CreateSecretRequest,
	DeleteSecretResponse,
	ProfileRequest,
	SecretMetadata
} from '$lib/types/connection.js';
import type { SaveState } from './lorebook.svelte.js';
import type { ProviderModel } from '$lib/types/schema.js';
import { SvelteMap } from 'svelte/reactivity';

export interface ProfileDraft {
	id: Snowflake;
	name: string;
	provider: string;
	model: string;
	secret_id: Snowflake | null;
	common_params: Record<string, JsonValue>;
	provider_params: Record<string, JsonValue>;
}

class ConnectionStore {
	private _api = $state<ApiClient>();

	get #api() {
    if (!this._api) this._api = useApi();
		return this._api;
	}

	profiles = $state<ConnectionProfile[]>();
	secrets = $state<SecretMetadata[]>();
	modelsByProvider = new SvelteMap<string, ProviderModel[]>();
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

	/**
	 * NOTE: requires a new api client method (and server route) backed by
	 * LlmProvider::list_models() — this will not compile until you add it:
	 *
	 *   providers: {
	 *     models: (provider: string) =>
	 *       http.get<Array<ProviderModel>>(`/api/v1/connections/providers/${provider}/models`)
	 *   }
	 *
	 * Failure (or a provider returning None) degrades to an empty list, so the
	 * model field simply has no suggestions.
	 */
	async loadModels(provider: string) {
		if (!provider || this.modelsByProvider.has(provider)) return;
		try {
			const models = null; //await this.#api.connection.providers.models(provider);
			this.modelsByProvider.set(provider, models ?? []);
		} catch (e) {
			console.error(e);
			this.modelsByProvider.set(provider, []);
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
				common_params: $state.snapshot(draft.common_params) as unknown as JsonValue,
				provider_params: $state.snapshot(draft.provider_params) as unknown as JsonValue
			};
			const saved = await this.#api.connection.profiles.update(draft.id, req);
			const i = this.profiles?.findIndex((p) => p.id === saved.id) ?? -1;
			if (this.profiles && i !== -1) this.profiles[i] = saved;
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

function buildDraft(base: ConnectionProfile): ProfileDraft {
	return {
		id: base.id,
		name: base.name,
		provider: base.provider,
		model: base.model,
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

export const connectionStore = new ConnectionStore();
