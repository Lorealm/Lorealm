import { PersistedRecord } from '#lib/stores/persisted.svelte.js';

/**
 * Whether shortcuts run at all. `auto` turns them off on touch-only devices;
 * `on` forces them for a tablet with a keyboard attached.
 */
export type KeybindMode = 'auto' | 'on' | 'off';

export interface KeybindSettings {
	mode: KeybindMode;
	overrides: Record<string, string[]>;
}

const DEFAULTS: KeybindSettings = {
	mode: 'auto',
	overrides: {}
};

class KeybindConfig {
	#record = new PersistedRecord<KeybindSettings>('keybinds', DEFAULTS);

	load(): Promise<void> {
		return this.#record.load();
	}

	get mode(): KeybindMode {
		return this.#record.current.mode;
	}

	get overrides(): Record<string, string[]> {
		return this.#record.current.overrides;
	}

	setMode(mode: KeybindMode): Promise<void> {
		return this.#record.set('mode', mode);
	}

	isOverridden(id: string): boolean {
		return id in this.overrides;
	}

	/** Replace a command's bindings. An empty list unbinds it. */
	setKeys(id: string, chords: string[]): Promise<void> {
		return this.#record.set('overrides', { ...this.overrides, [id]: chords });
	}

	reset(id: string): Promise<void> {
		const next = { ...this.overrides };
		delete next[id];
		return this.#record.set('overrides', next);
	}

	resetAll(): Promise<void> {
		return this.#record.set('overrides', {});
	}
}

export const keybindConfig = new KeybindConfig();
