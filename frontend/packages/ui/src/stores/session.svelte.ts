import type { Route } from '$lib/navigation/routes.js';

/**
 * What the app remembers for as long as this tab stays open.
 */
export interface SessionState {
	/** The view to return to when the tab is reloaded. */
	lastRoute: Route | null;
}

const DEFAULTS: SessionState = {
	lastRoute: null
};

/**
 * One JSON record in `sessionStorage`, mirrored into reactive state.
 *
 * Unlike {@link PersistedRecord} this reads at construction rather than through
 * a `load()`
 */
class SessionRecord<T extends object> {
	readonly #key: string;

	#value: T = $state()!;

	constructor(key: string, defaults: T) {
		this.#value = { ...defaults, ...this.#read(key) };
		this.#key = key;
	}

	get current(): T {
		return this.#value;
	}

	set<K extends keyof T>(key: K, value: T[K]): void {
		this.#value[key] = value;
		this.#write();
	}

	#read(key: string): Partial<T> | null {
		try {
			const raw = sessionStorage.getItem(key);
			return raw === null ? null : (JSON.parse(raw) as Partial<T>);
		} catch {
			// No storage, or a corrupt entry: fall back to the defaults.
			return null;
		}
	}

	#write(): void {
		try {
			sessionStorage.setItem(this.#key, JSON.stringify($state.snapshot(this.#value)));
		} catch {
			// Storage disabled or full.
		}
	}
}

export const session = new SessionRecord<SessionState>('lorealm:session', DEFAULTS);
