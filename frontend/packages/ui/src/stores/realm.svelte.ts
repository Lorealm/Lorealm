import { useApi } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';
import type { Realm, RealmSummary } from '#lib/types/realm.js';
import { showApiError } from './errorActions.js';
import { t } from '#lib/stores/i18n.svelte.js';

class RealmStore {
	get #api() {
		return useApi();
	}

	summaries = $state<Array<RealmSummary>>([]);
	/** The realm currently open. */
	realm = $state<Realm | null>(null);

	async loadList() {
		this.summaries = await this.#api.realms.list();
	}

	async deleteRealm(id: Snowflake) {
		const prev = [...this.summaries];
		this.summaries = this.summaries.filter((r) => r.id !== id);
		if (this.realm?.id === id) this.closeRealm();
		try {
			await this.#api.realms.delete(id);
		} catch (e) {
			this.summaries = prev;
			showApiError(e, { title: t.errors.failed.deleteRealm, durationMs: 5000 });
		}
	}

	async openRealm(id: Snowflake) {
		this.realm = await this.#api.realms.get(id);
	}

	closeRealm() {
		this.realm = null;
	}
}

export const realmStore = new RealmStore();
