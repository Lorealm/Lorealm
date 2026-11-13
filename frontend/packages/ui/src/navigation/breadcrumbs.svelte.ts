import { characterStore } from '#lib/stores/character.svelte.js';
import { connectionStore } from '#lib/stores/connection.svelte.js';
import { lorebookStore } from '#lib/stores/lorebook.svelte.js';
import { personaStore } from '#lib/stores/persona.svelte.js';
import { marketplaceStore, shownVersion } from '#lib/stores/marketplace.svelte.js';
import { pluginsStore } from '#lib/stores/plugins.svelte.js';
import { realmStore } from '#lib/stores/realm.svelte.js';
import { templateStore } from '#lib/stores/template.svelte.js';
import { moduleStore } from '#lib/stores/modules.svelte.js';
import { toolStore } from '#lib/stores/tools.svelte.js';
import { userWidgetStore } from '#lib/stores/userWidgets.svelte.js';
import type { Snowflake } from '#lib/types/core.js';
import { navigator } from './navigator.js';
import {
	SECRETS,
	type Route,
	type ServerSection,
	type StudioSection,
	type UserSection,
	type ViewName
} from './routes.js';

export interface Crumb {
	label: string;
	go?: () => void;
}

const PENDING = '…';

export const VIEW_LABEL: Record<ViewName, string> = {
	write: 'Write',
	characters: 'Characters',
	realms: 'Realms',
	connections: 'Connections',
	studio: 'Studio',
	plugins: 'Plugins',
	settings: 'Settings'
};

export const STUDIO_LABEL: Record<StudioSection, string> = {
	realm: 'Realm',
	characters: 'Characters',
	lorebooks: 'Lorebooks',
	templates: 'Templates',
	tools: 'Tools',
	modules: 'Modules',
	personas: 'Personas',
	widgets: 'Widgets'
};

const SCOPE_LABEL: Record<'user' | 'server', string> = {
	user: 'You',
	server: 'Server'
};

const USER_LABEL: Record<UserSection, string> = {
	profile: 'Profile',
	tokenizers: 'Tokenizers',
	plugins: 'Plugins',
	appearance: 'Appearance',
	editor: 'Editor',
	inspector: 'Inspector',
	keybinds: 'Keybinds'
};

const SERVER_LABEL: Record<ServerSection, string> = {
	hosting: 'Hosting',
	storage: 'Storage',
	sessions: 'Sessions',
	users: 'Users',
	accounts: 'Accounts',
	roles: 'Roles',
	characters: 'Characters',
	tokenizers: 'Tokenizers',
	tasks: 'Tasks'
};

interface Named {
	id: Snowflake;
	name: string;
	display_name?: string | null;
}

function nameIn(list: Named[] | undefined, id: Snowflake): string {
	const found = list?.find((item) => item.id === id);
	if (!found) return PENDING;
	return found.display_name || found.name;
}

function studioName(section: StudioSection, id: Snowflake): string | null {
	switch (section) {
		case 'characters':
			return nameIn(characterStore.metadataList, id);
		case 'lorebooks':
			return nameIn(lorebookStore.metadataList, id);
		case 'templates':
			return nameIn(templateStore.metadataList, id);
		case 'tools':
			return nameIn([...(toolStore.loadedTools ?? []), ...(toolStore.builtinTools ?? [])], id);
		case 'modules':
			return nameIn(moduleStore.loadedModules, id);
		case 'personas':
			return nameIn(personaStore.loadedPersonas, id);
		case 'widgets':
			return nameIn(userWidgetStore.loadedWidgets, id);
		case 'realm':
			return null;
	}
}

/**
 * The trail to the current route, root first.
 */
export function crumbsFor(route: Route): Crumb[] {
	switch (route.view) {
		case 'write':
		case 'characters':
			return [{ label: VIEW_LABEL[route.view] }];

		case 'realms': {
			if (route.tab === 'blueprints') {
				return [
					{ label: VIEW_LABEL.realms, go: () => navigator.start().realms() },
					{ label: 'Blueprints' }
				];
			}
			if (route.id === undefined) return [{ label: VIEW_LABEL.realms }];
			return [
				{ label: VIEW_LABEL.realms, go: () => navigator.start().realms() },
				{ label: nameIn(realmStore.summaries, route.id) }
			];
		}

		case 'plugins': {
			const plugins = { label: VIEW_LABEL.plugins, go: () => navigator.start().plugins() };
			if (route.tab === 'marketplace') {
				if (route.id === undefined) return [plugins, { label: 'Marketplace' }];
				const listed = marketplaceStore.find(route.id);
				return [
					plugins,
					{ label: 'Marketplace', go: () => navigator.start().marketplace() },
					{ label: listed ? shownVersion(listed).name : PENDING }
				];
			}
			if (route.id === undefined) return [{ label: VIEW_LABEL.plugins }];
			const plugin = pluginsStore.plugins.find((p) => p.handle === route.id);
			return [plugins, { label: plugin?.name ?? PENDING }];
		}

		case 'connections': {
			if (route.id === undefined) return [{ label: VIEW_LABEL.connections }];
			const label = route.id === SECRETS ? 'API keys' : nameIn(connectionStore.profiles, route.id);
			return [
				{ label: VIEW_LABEL.connections, go: () => navigator.start().connections() },
				{ label }
			];
		}

		case 'studio': {
			const section = route.section;
			const name = route.id === undefined ? null : studioName(section, route.id);
			return [
				{ label: VIEW_LABEL.studio },
				{
					label: STUDIO_LABEL[section],
					go: name === null ? undefined : () => navigator.start().studio()[section]()
				},
				...(name === null ? [] : [{ label: name }])
			];
		}

		case 'settings': {
			const section =
				route.scope === 'user' ? USER_LABEL[route.section] : SERVER_LABEL[route.section];
			if (route.scope === 'user' && route.id !== undefined) {
				const plugin = pluginsStore.plugins.find((p) => p.handle === route.id);
				return [
					{ label: VIEW_LABEL.settings },
					{ label: SCOPE_LABEL.user },
					{ label: section, go: () => navigator.start().settings().user().plugins() },
					{ label: plugin?.name ?? PENDING }
				];
			}
			return [
				{ label: VIEW_LABEL.settings },
				{ label: SCOPE_LABEL[route.scope] },
				{ label: section }
			];
		}
	}
}
