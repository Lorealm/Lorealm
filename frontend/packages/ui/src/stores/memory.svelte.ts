import { PersistedRecord } from './persisted.svelte.js';

/**
 * What the app remembers about the last session.
 */
export interface Memory {
	/** Prefills the login form. */
	lastUsername: string;
}

const DEFAULTS: Memory = {
	lastUsername: ''
};

export const memory = new PersistedRecord<Memory>('memory', DEFAULTS);
