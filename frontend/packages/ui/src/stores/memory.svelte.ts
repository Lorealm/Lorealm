import { PersistedRecord } from './persisted.svelte.js';

/** The order the Characters view lists in. */
export type CharacterSort = 'name-asc' | 'name-desc' | 'updated' | 'created';

/**
 * What the app remembers about the last session.
 */
export interface Memory {
	/** Prefills the login form. */
	lastUsername: string;

	/** The sort the Characters view opens with. */
	charactersSort: CharacterSort;
}

const DEFAULTS: Memory = {
	lastUsername: '',
	charactersSort: 'name-asc'
};

export const memory = new PersistedRecord<Memory>('memory', DEFAULTS);
