import { PersistedRecord } from './persisted.svelte.js';

export interface ClientSettings {
	/** Offer entry ids from the other active lorebooks. */
	weaverSuggestCrossBookIds: boolean;

	/** Let cmd/ctrl-click on a cross-book id switch to the book that owns it. */
	weaverFollowCrossBookIds: boolean;

	/** Automatically expand the advanced section of a lorebook entry. */
	autoExpandLorebookAdvanced: boolean;

	/** Give `{#- … -#}` trim markers their own colour, apart from the delimiter. */
	weaverHighlightTrimMarkers: boolean;

	/** Interface language, or `null` to follow the device. */
	locale: string | null;
}

const DEFAULTS: ClientSettings = {
	weaverSuggestCrossBookIds: true,
	weaverFollowCrossBookIds: true,
	autoExpandLorebookAdvanced: false,
	weaverHighlightTrimMarkers: true,
	locale: null
};

export const settings = new PersistedRecord<ClientSettings>('settings', DEFAULTS);
