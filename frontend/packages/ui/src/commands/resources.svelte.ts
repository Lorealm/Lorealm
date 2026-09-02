import { SvelteMap } from 'svelte/reactivity';
import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import type { PickableResource } from '$lib/types/tools.js';

/** One thing a resource-typed argument could name. */
export interface ResourceOption {
	/** The Snowflake that ends up in the command argument. */
	id: Snowflake;
	/** What the user searches by. */
	name: string;
	/** Disambiguates two things sharing a name (when enforced by a unique constraint) */
	hint?: string;
}

/** How to name and list one kind. */
export interface ResourceLoader {
	label: string;
	load: () => Promise<ResourceOption[]>;
}

/** What each kind is called in the picker, and how to list it. */
const KINDS: Record<PickableResource, ResourceLoader> = {
	character: {
		label: 'Character',
		load: async () =>
			(await useApi().characters.list()).map((c) => ({
				id: c.id,
				name: c.name,
				hint: c.display_name
			}))
	},
	persona: {
		label: 'Persona',
		load: async () =>
			(await useApi().personas.list()).map((p) => ({
				id: p.id,
				name: p.name,
				hint: p.display_name ?? undefined
			}))
	},
	connection_profile: {
		label: 'Connection profile',
		load: async () =>
			(await useApi().connection.profiles.list()).map((p) => ({
				id: p.id,
				name: p.name,
				hint: p.model ?? p.provider
			}))
	},
	context_template: {
		label: 'Context template',
		load: async () => (await useApi().templates.list()).map((t) => ({ id: t.id, name: t.name }))
	},
	lorebook: {
		label: 'Lorebook',
		load: async () => (await useApi().lorebooks.list()).map((l) => ({ id: l.id, name: l.name }))
	},
	tokenizer: {
		label: 'Tokenizer',
		load: async () => (await useApi().tokenizers.list()).map((t) => ({ id: t.id, name: t.name }))
	},
	user_tool: {
		label: 'Tool',
		load: async () =>
			(await useApi().tools.list()).map((t) => ({
				id: t.id,
				name: t.name,
				hint: t.display_name ?? undefined
			}))
	},
	user_widget: {
		label: 'Widget',
		load: async () => (await useApi().userWidgets.list()).map((w) => ({ id: w.id, name: w.name }))
	},
	realm: {
		label: 'Realm',
		load: async () => (await useApi().realms.list()).map((r) => ({ id: r.id, name: r.name }))
	}
};

export const RESOURCE_KINDS = Object.keys(KINDS) as PickableResource[];

export function resourceLabel(kind: PickableResource): string {
	return KINDS[kind].label;
}

/**
 * Listings, fetched once per kind and kept.
 */
export class ResourceCatalog {
	#kinds: Record<PickableResource, ResourceLoader>;

	constructor(kinds: Record<PickableResource, ResourceLoader> = KINDS) {
		this.#kinds = kinds;
	}

	/** Reactive: a load finishing has to re-run the composer's `$derived`,
	 *  or the popup stays empty until the next keystroke. */
	#byKind = new SvelteMap<PickableResource, ResourceOption[]>();

	/**
	 * Kinds whose load has finished, successfully or not.
	 */
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	#resolved = new Set<PickableResource>();
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	#inFlight = new Map<PickableResource, Promise<void>>();

	/**
	 * What is loaded for `kind`, or nothing yet.
	 */
	options(kind: PickableResource): ResourceOption[] {
		return this.#byKind.get(kind) ?? [];
	}

	/** Load a kind once. Repeated calls after it settles do nothing. */
	prime(kind: PickableResource): Promise<void> {
		if (this.#resolved.has(kind)) return Promise.resolve();
		const running = this.#inFlight.get(kind);
		if (running) return running;

		const load = this.#kinds[kind]
			.load()
			.then((options) => {
				this.#resolved.add(kind);
				this.#byKind.set(kind, options);
			})
			.catch((e) => {
				this.#resolved.add(kind);
				console.warn(`Failed to load ${kind} options`, e);
			})
			.finally(() => {
				this.#inFlight.delete(kind);
			});

		this.#inFlight.set(kind, load);
		return load;
	}

	/** Entries whose name or hint contains `query`, best-prefix first. */
	search(kind: PickableResource, query: string): ResourceOption[] {
		const needle = query.trim().toLowerCase();
		const all = this.options(kind);
		if (!needle) return all;
		return all
			.filter(
				(o) =>
					o.name.toLowerCase().includes(needle) || (o.hint ?? '').toLowerCase().includes(needle)
			)
			.sort((a, b) => {
				const byPrefix =
					Number(b.name.toLowerCase().startsWith(needle)) -
					Number(a.name.toLowerCase().startsWith(needle));
				return byPrefix || a.name.localeCompare(b.name);
			});
	}

	/** The name behind an id, for showing what an argument points at. */
	nameOf(kind: PickableResource, id: string): string | undefined {
		return this.#byKind.get(kind)?.find((o) => String(o.id) === id)?.name;
	}
}

export const resourceCatalog = new ResourceCatalog();
