/**
 * Weaver editor preferences.
 *
 * TODO: Save to local user prefs when that eventually exists
 */
class WeaverEditorPrefs {
	/** Offer entry ids from the other active lorebooks */
	suggestCrossBookIds = $state(true);

	/** Let cmd/ctrl-click on a cross-book id switch to the book that owns it. */
	followCrossBookIds = $state(true);
}

export const weaverEditorPrefs = new WeaverEditorPrefs();
