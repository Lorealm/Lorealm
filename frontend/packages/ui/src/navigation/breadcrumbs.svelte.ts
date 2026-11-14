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
import { t } from '#lib/stores/i18n.svelte.js';
import { navigator } from './navigator.js';
import { SECRETS, type Route, type StudioSection } from './routes.js';

export interface Crumb {
	label: string;
	go?: () => void;
}

const PENDING = '…';

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
			return [{ label: t.nav.views[route.view] }];

		case 'realms': {
			if (route.tab === 'blueprints') {
				return [
					{ label: t.nav.views.realms, go: () => navigator.start().realms() },
					{ label: t.nav.blueprints }
				];
			}
			if (route.id === undefined) return [{ label: t.nav.views.realms }];
			return [
				{ label: t.nav.views.realms, go: () => navigator.start().realms() },
				{ label: nameIn(realmStore.summaries, route.id) }
			];
		}

		case 'plugins': {
			const plugins = { label: t.nav.views.plugins, go: () => navigator.start().plugins() };
			if (route.tab === 'marketplace') {
				if (route.id === undefined) return [plugins, { label: t.nav.marketplace }];
				const listed = marketplaceStore.find(route.id);
				return [
					plugins,
					{ label: t.nav.marketplace, go: () => navigator.start().marketplace() },
					{ label: listed ? shownVersion(listed).name : PENDING }
				];
			}
			if (route.id === undefined) return [{ label: t.nav.views.plugins }];
			const plugin = pluginsStore.plugins.find((p) => p.handle === route.id);
			return [plugins, { label: plugin?.name ?? PENDING }];
		}

		case 'connections': {
			if (route.id === undefined) return [{ label: t.nav.views.connections }];
			const label =
				route.id === SECRETS ? t.nav.apiKeys : nameIn(connectionStore.profiles, route.id);
			return [
				{ label: t.nav.views.connections, go: () => navigator.start().connections() },
				{ label }
			];
		}

		case 'studio': {
			const section = route.section;
			const name = route.id === undefined ? null : studioName(section, route.id);
			return [
				{ label: t.nav.views.studio },
				{
					label: t.nav.studio[section],
					go: name === null ? undefined : () => navigator.start().studio()[section]()
				},
				...(name === null ? [] : [{ label: name }])
			];
		}

		case 'settings': {
			const section =
				route.scope === 'user' ? t.nav.user[route.section] : t.nav.server[route.section];
			if (route.scope === 'user' && route.id !== undefined) {
				const plugin = pluginsStore.plugins.find((p) => p.handle === route.id);
				return [
					{ label: t.nav.views.settings },
					{ label: t.nav.scopes.user },
					{ label: section, go: () => navigator.start().settings().user().plugins() },
					{ label: plugin?.name ?? PENDING }
				];
			}
			return [
				{ label: t.nav.views.settings },
				{ label: t.nav.scopes[route.scope] },
				{ label: section }
			];
		}
	}
}
