import { usePlatform } from '#lib/api/client.js';
import { showError } from './errorToast.svelte.js';
import { t } from '#lib/stores/i18n.svelte.js';

// eslint-disable-next-line @typescript-eslint/no-unused-vars
import type { LocalStore } from '#lib/core/api.js'; // Imported for doc links

/**
 * One JSON record in the platform's {@link LocalStore}, mirrored into reactive
 * state.
 */
export class PersistedRecord<T extends object> {
	readonly #key: string;
	readonly #defaults: T;

	#value: T = $state()!;
	loaded = $state(false);

	constructor(key: string, defaults: T) {
		this.#key = key;
		this.#defaults = defaults;
		this.#value = { ...defaults };
	}

	get current(): T {
		return this.#value;
	}

	async load(): Promise<void> {
		const stored = await usePlatform().store.get<Partial<T>>(this.#key);

		this.#value = { ...this.#defaults, ...stored };
		this.loaded = true;
	}

	async set<K extends keyof T>(key: K, value: T[K]): Promise<void> {
		this.#value[key] = value;
		await this.#save();
	}

	async reset(): Promise<void> {
		this.#value = { ...this.#defaults };
		await usePlatform().store.delete(this.#key);
	}

	async #save(): Promise<void> {
		try {
			await usePlatform().store.set(this.#key, $state.snapshot(this.#value));
		} catch (e) {
			showError(e, { title: t.errors.failed.persist({ key: this.#key }) });
		}
	}
}
