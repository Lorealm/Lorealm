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
