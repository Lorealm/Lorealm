import type { Snowflake } from '$lib/types/core.js';
import type { RealmResourceKind } from '$lib/types/realm.js';
import { characterStore } from '$lib/stores/character.svelte.js';
import { connectionStore } from '$lib/stores/connection.svelte.js';
import { lorebookStore } from '$lib/stores/lorebook.svelte.js';
import { moduleStore } from '$lib/stores/modules.svelte.js';
import { personaStore } from '$lib/stores/persona.svelte.js';
import { templateStore } from '$lib/stores/template.svelte.js';
import { toolStore } from '$lib/stores/tools.svelte.js';
import { userWidgetStore } from '$lib/stores/userWidgets.svelte.js';

/** One bindable thing, flattened out of whichever store owns it. */
export interface ResourceItem {
	kind: RealmResourceKind;
	id: Snowflake;
	name: string;
	/** Secondary line — whatever disambiguates two same-named entries. */
	hint?: string;
}

export const RESOURCE_KINDS: RealmResourceKind[] = [
	'character',
	'lorebook',
	'template',
	'persona',
	'tool',
	'connection_profile',
	'widget',
	'module'
];

/**
 * The one place the asset browser and the radio field agree on what exists.
 */
const SOURCES: Record<
	RealmResourceKind,
	{ load: () => Promise<unknown>; read: () => ResourceItem[] }
> = {
	character: {
		load: () => characterStore.loadList(),
		read: () =>
			characterStore.metadataList.map((c) => ({
				kind: 'character' as const,
				id: c.id,
				name: c.display_name || c.name,
				hint: c.tags.join(', ') || undefined
			}))
	},
	lorebook: {
		load: () => lorebookStore.loadList(),
		read: () =>
			(lorebookStore.metadataList ?? []).map((l) => ({
				kind: 'lorebook' as const,
				id: l.id,
				name: l.name,
				hint: `${l.entry_count} entries`
			}))
	},
	template: {
		load: () => templateStore.loadList(),
		read: () =>
			(templateStore.metadataList ?? []).map((t) => ({
				kind: 'template' as const,
				id: t.id,
				name: t.name,
				hint: t.description || undefined
			}))
	},
	persona: {
		load: () => personaStore.loadPersonas(),
		read: () =>
			(personaStore.loadedPersonas ?? []).map((p) => ({
				kind: 'persona' as const,
				id: p.id,
				name: p.display_name || p.name,
				hint: p.pseudonym || undefined
			}))
	},
	tool: {
		load: () => toolStore.loadTools(),
		read: () =>
			(toolStore.loadedTools ?? []).map((t) => ({
				kind: 'tool' as const,
				id: t.id,
				name: t.display_name || t.name,
				hint: t.description || undefined
			}))
	},
	connection_profile: {
		load: () => connectionStore.loadProfiles(),
		read: () =>
			(connectionStore.profiles ?? []).map((p) => ({
				kind: 'connection_profile' as const,
				id: p.id,
				name: p.name,
				hint: p.model || undefined
			}))
	},
	widget: {
		load: () => userWidgetStore.loadWidgets(),
		read: () =>
			(userWidgetStore.loadedWidgets ?? []).map((w) => ({
				kind: 'widget' as const,
				id: w.id,
				name: w.name,
				hint: w.description || undefined
			}))
	},
	module: {
		load: () => moduleStore.loadModules(),
		read: () =>
			(moduleStore.loadedModules ?? []).map((m) => ({
				kind: 'module' as const,
				id: m.id,
				name: m.name,
				hint: m.description || undefined
			}))
	}
};

const requested: Partial<Record<RealmResourceKind, true>> = {};

/** Fetch each kind's list once per session. Failures are the stores' to report. */
export function ensureLoaded(kinds: RealmResourceKind[]) {
	for (const kind of kinds) {
		if (requested[kind]) continue;
		requested[kind] = true;
		// A failed load should not be remembered as done.
		SOURCES[kind].load().catch(() => delete requested[kind]);
	}
}

export function itemsOf(kind: RealmResourceKind): ResourceItem[] {
	return SOURCES[kind].read();
}

/** Fetch each kind's list again, after something outside the stores added to it. */
export function reload(kinds: RealmResourceKind[]) {
	for (const kind of kinds) {
		requested[kind] = true;
		SOURCES[kind].load().catch(() => delete requested[kind]);
	}
}
