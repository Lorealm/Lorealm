import type { Snowflake } from '$lib/types/core.js';

/** A workspace within the Studio. */
export type StudioSection =
	| 'realm'
	| 'characters'
	| 'lorebooks'
	| 'templates'
	| 'tools'
	| 'personas'
	| 'widgets';

/** The sections under "You" that live on this device only, never on the account. */
export type ClientSection = 'editor' | 'inspector' | 'keybinds';

/** Everything about you: the account itself, plus this device's preferences. */
export type UserSection = 'profile' | 'tokenizers' | ClientSection;

export type ServerSection =
	| 'hosting'
	| 'storage'
	| 'sessions'
	| 'users'
	| 'characters'
	| 'tokenizers'
	| 'tasks';

/** Stands in for a profile id when the Connections view shows the key manager. */
export const SECRETS = 'secrets';

export type Route =
	| { view: 'write' }
	| { view: 'characters' }
	| { view: 'realms'; id?: Snowflake }
	| { view: 'connections'; id?: Snowflake | typeof SECRETS }
	| { view: 'studio'; section: StudioSection; id?: Snowflake }
	| { view: 'settings'; scope: 'user'; section: UserSection }
	| { view: 'settings'; scope: 'server'; section: ServerSection };

/** The top-level view a route belongs to. What the nav bar highlights. */
export type ViewName = Route['view'];

export const HOME: Route = { view: 'write' };

export function routeKey(route: Route): string {
	switch (route.view) {
		case 'studio':
			return `studio:${route.section}`;
		case 'settings':
			return `settings:${route.scope}:${route.section}`;
		default:
			return route.view;
	}
}

export function routeId(route: Route): string | undefined {
	return 'id' in route ? route.id : undefined;
}

export function sameRoute(a: Route, b: Route): boolean {
	return routeKey(a) === routeKey(b) && routeId(a) === routeId(b);
}
