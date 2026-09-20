import { characterStore } from '$lib/stores/character.svelte.js';
import { connectionStore } from '$lib/stores/connection.svelte.js';
import { currentUser } from '$lib/stores/currentUser.svelte.js';
import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
import { personaStore } from '$lib/stores/persona.svelte.js';
import { realmStore } from '$lib/stores/realm.svelte.js';
import { configStore } from '$lib/stores/serverConfig.svelte.js';
import { templateStore } from '$lib/stores/template.svelte.js';
import { tokenizerStore } from '$lib/stores/tokenizer.svelte.js';
import { toolStore } from '$lib/stores/tools.svelte.js';
import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';
import { routeId, routeKey, SECRETS, type Route } from './routes.js';

/**
 * A list every view of a kind shares.
 */
class Resource {
	#pending: Promise<void> | null = null;

	constructor(private readonly run: () => Promise<unknown>) {}

	ensure(): Promise<void> {
		return (this.#pending ??= Promise.resolve(this.run())
			.then(() => undefined)
			.catch((e) => {
				this.#pending = null;
				throw e;
			}));
	}
}

const resources = {
	characterList: new Resource(() => characterStore.loadList()),
	lorebookList: new Resource(() => lorebookStore.loadList()),
	lorebookActive: new Resource(() => lorebookStore.loadActive()),
	templateList: new Resource(() => templateStore.loadList()),
	templateActive: new Resource(() => templateStore.loadActive()),
	templateSources: new Resource(() => templateStore.loadSources()),
	toolList: new Resource(() => toolStore.loadTools()),
	builtinTools: new Resource(() => toolStore.loadBuiltins()),
	widgetList: new Resource(() => userWidgetStore.loadWidgets()),
	personaList: new Resource(() => personaStore.loadPersonas()),
	personaActive: new Resource(() => personaStore.loadActive()),
	personaOverrides: new Resource(() => personaStore.loadOverrides()),
	connectionProfiles: new Resource(() => connectionStore.loadProfiles()),
	connectionSecrets: new Resource(() => connectionStore.loadSecrets()),
	connectionActive: new Resource(() => connectionStore.getActiveProfile()),
	realmList: new Resource(() => realmStore.loadList()),
	currentUser: new Resource(() => currentUser.load()),
	serverConfig: new Resource(() => configStore.load()),
	tokenizerList: new Resource(() => tokenizerStore.load())
} satisfies Record<string, Resource>;

type ResourceKey = keyof typeof resources;

/** Everything a realm binds to */
const CATALOG: ResourceKey[] = [
	'characterList',
	'lorebookList',
	'templateList',
	'toolList',
	'builtinTools',
	'widgetList',
	'personaList',
	'connectionProfiles'
];

interface RouteSpec {
	/** Must be in memory before the view renders. */
	needs?: ResourceKey[];
	/** Opens the route's selected resource. Runs after `needs`. */
	focus?: (id: string) => Promise<unknown> | void;
	/** Tears the edit session down when the route drops its selection or is left. */
	blur?: () => void;
}

/** Every Studio route renders `StudioNav`, which lists the realms. */
const studio = (spec: RouteSpec): RouteSpec => ({
	...spec,
	needs: ['realmList', ...(spec.needs ?? [])]
});

/** What each destination needs */
const SPECS: Record<string, RouteSpec> = {
	write: {
		needs: ['widgetList', 'characterList', 'personaList', 'personaActive', 'personaOverrides']
	},

	characters: { needs: ['characterList'] },

	realms: {
		needs: ['realmList', ...CATALOG],
		focus: (id) => realmStore.openRealm(id),
		blur: () => realmStore.closeRealm()
	},

	connections: {
		needs: ['connectionProfiles', 'connectionSecrets', 'connectionActive'],
		// The key manager occupies the selection slot without being a profile.
		focus: (id) =>
			id === SECRETS ? connectionStore.stopEditing() : connectionStore.startEditing(id),
		blur: () => connectionStore.stopEditing()
	},

	'studio:realm': studio({}),

	'studio:characters': studio({
		needs: ['characterList', 'widgetList'],
		focus: (id) => characterStore.startEditing(id),
		blur: () => characterStore.stopEditing()
	}),

	'studio:lorebooks': studio({
		needs: ['lorebookList', 'lorebookActive'],
		focus: async (id) => {
			await lorebookStore.startEditing(id);
			// Cross-book id resolution reads from the other active books.
			await lorebookStore.loadActiveBooks();
		},
		blur: () => lorebookStore.stopEditing()
	}),

	'studio:templates': studio({
		needs: ['templateList', 'templateActive', 'templateSources'],
		focus: (id) => templateStore.startEditing(id),
		blur: () => templateStore.stopEditing()
	}),

	'studio:tools': studio({
		needs: ['toolList', 'builtinTools'],
		// Builtins and the user's own tools share one id space and one slot.
		focus: (id) =>
			toolStore.builtinTools?.some((t) => t.id === id)
				? toolStore.viewBuiltin(id)
				: toolStore.startEditing(id),
		blur: () => toolStore.stopEditing()
	}),

	'studio:personas': studio({
		needs: ['personaList', 'personaActive', 'personaOverrides', 'characterList'],
		focus: (id) => personaStore.startEditing(id),
		blur: () => personaStore.stopEditing()
	}),

	'studio:widgets': studio({ needs: ['widgetList', 'toolList', 'builtinTools'] }),

	'settings:user:profile': {
		needs: ['currentUser', 'characterList', 'lorebookList', 'realmList']
	},
	'settings:user:tokenizers': { needs: ['tokenizerList'] },

	'settings:server:hosting': { needs: ['serverConfig'] },
	'settings:server:storage': { needs: ['serverConfig'] },
	'settings:server:sessions': { needs: ['serverConfig'] },
	'settings:server:users': { needs: ['serverConfig'] },
	'settings:server:characters': { needs: ['serverConfig'] },
	'settings:server:tokenizers': { needs: ['serverConfig'] },
	'settings:server:tasks': { needs: ['serverConfig'] }
};

export async function applyRoute(
	previous: Route | null,
	next: Route,
	stillCurrent: () => boolean
): Promise<void> {
	const nextKey = routeKey(next);
	const id = routeId(next);

	if (previous && (routeKey(previous) !== nextKey || id === undefined)) {
		SPECS[routeKey(previous)]?.blur?.();
	}

	const spec = SPECS[nextKey];
	if (!spec) return;

	await Promise.all((spec.needs ?? []).map((key) => resources[key].ensure()));
	if (!stillCurrent() || id === undefined) return;

	await spec.focus?.(id);
}
