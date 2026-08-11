import { PersistedRecord } from './persisted.svelte.js';

/**
 * Preferences the user sets from the Client tab of the settings window. Local
 * to this device; the server never sees them.
 */
export interface ClientSettings {
	/** Offer entry ids from the other active lorebooks. */
	weaverSuggestCrossBookIds: boolean;

	/** Let cmd/ctrl-click on a cross-book id switch to the book that owns it. */
	weaverFollowCrossBookIds: boolean;
}

const DEFAULTS: ClientSettings = {
	weaverSuggestCrossBookIds: true,
	weaverFollowCrossBookIds: true
};

export const settings = new PersistedRecord<ClientSettings>('settings', DEFAULTS);
