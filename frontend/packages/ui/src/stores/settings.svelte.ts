export enum SettingsScope {
	User,
	Server
}

export enum UserSection {
	Tokenizers
}

export enum ServerSection {
	Hosting,
	Storage,
	Sessions,
	Users,
	Characters,
	Tokenizers,
	Tasks
}

class SettingsNavStore {
	scope = $state(SettingsScope.User);
	userSection = $state(UserSection.Tokenizers);
	serverSection = $state(ServerSection.Hosting);

	setScope(scope: SettingsScope) {
		this.scope = scope;
	}
	setUserSection(section: UserSection) {
		this.userSection = section;
	}
	setServerSection(section: ServerSection) {
		this.serverSection = section;
	}
}

export const settingsNav = new SettingsNavStore();

/**
 * Weaver editor preferences.
 *
 * TODO: there is no local user-settings infrastructure yet, so these are
 * hardcoded stubs with no UI behind them. Move them onto real persisted
 * settings once that exists.
 */
class WeaverEditorPrefs {
	/**
	 * Offer entry ids from the other active lorebooks, not only the one being
	 * edited. Mirrors context_weaver's `DefaultIdResolver`, which falls back to
	 * the remaining active books when an id misses locally.
	 */
	suggestCrossBookIds = $state(true);

	/** Let cmd/ctrl-click on a cross-book id switch to the book that owns it. */
	followCrossBookIds = $state(true);
}

export const weaverEditorPrefs = new WeaverEditorPrefs();
