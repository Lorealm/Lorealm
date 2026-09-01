import { session } from '$lib/stores/session.svelte.js';
import { applyRoute } from './loader.svelte.js';
import {
	HOME,
	sameRoute,
	type ClientSection,
	type Route,
	type ServerSection,
	type StudioSection,
	type UserSection
} from './routes.js';

export type RouteStatus = 'ready' | 'loading' | 'error';

class Router {
	#stack = $state<Route[]>([HOME]);
	#index = $state(0);

	/** Bumped per navigation */
	#generation = 0;
	/** The stages of one `navigator` chain */
	#pending: { previous: Route | null; route: Route } | null = null;
	/** Where to land when the next entry fails. Only a restored route sets it. */
	#fallback: Route | null = null;

	/** Sections to return to when a destination names a view but not a section. */
	#studioSection: StudioSection = 'lorebooks';
	#settingsScope: 'client' | 'user' | 'server' = 'user';
	#clientSection: ClientSection = 'editor';
	#userSection: UserSection = 'tokenizers';
	#serverSection: ServerSection = 'hosting';

	status = $state<RouteStatus>('ready');
	error = $state<string | null>(null);

	get current(): Route {
		return this.#stack[this.#index];
	}
	get canGoBack(): boolean {
		return this.#index > 0;
	}
	get canGoForward(): boolean {
		return this.#index < this.#stack.length - 1;
	}

	go(route: Route, replace = false) {
		if (sameRoute(this.current, route)) return;
		const previous = this.current;

		if (replace) this.#stack[this.#index] = route;
		else {
			this.#stack = [...this.#stack.slice(0, this.#index + 1), route];
			this.#index = this.#stack.length - 1;
		}

		this.#remember(route);
		this.#schedule(previous, route);
	}

	back() {
		if (!this.canGoBack) return;
		const previous = this.current;
		this.#index -= 1;
		this.#schedule(previous, this.current);
	}

	forward() {
		if (!this.canGoForward) return;
		const previous = this.current;
		this.#index += 1;
		this.#schedule(previous, this.current);
	}

	enter() {
		this.#schedule(null, this.current);
	}

	/**
	 * Enter at the view this tab was on before it reloaded.
	 */
	restore() {
		const saved = session.current.lastRoute;
		if (saved) {
			this.#stack = [saved];
			this.#index = 0;
			this.#remember(saved);
			if ('id' in saved && saved.id !== undefined) this.#fallback = { ...saved, id: undefined };
		}
		this.enter();
	}

	reset() {
		this.#stack = [HOME];
		this.#index = 0;
		this.status = 'ready';
		this.error = null;
	}

	recallStudio(): Route {
		return { view: 'studio', section: this.#studioSection };
	}
	recallSettings(): Route {
		switch (this.#settingsScope) {
			case 'client':
				return this.recallClient();
			case 'server':
				return this.recallServer();
			default:
				return this.recallUser();
		}
	}
	recallClient(): Route {
		return { view: 'settings', scope: 'client', section: this.#clientSection };
	}
	recallUser(): Route {
		return { view: 'settings', scope: 'user', section: this.#userSection };
	}
	recallServer(): Route {
		return { view: 'settings', scope: 'server', section: this.#serverSection };
	}

	#remember(route: Route) {
		if (route.view === 'studio') this.#studioSection = route.section;
		if (route.view === 'settings') {
			this.#settingsScope = route.scope;
			if (route.scope === 'client') this.#clientSection = route.section;
			else if (route.scope === 'user') this.#userSection = route.section;
			else this.#serverSection = route.section;
		}
	}

	#schedule(previous: Route | null, route: Route) {
		if (this.#pending) {
			this.#pending.route = route;
			return;
		}
		this.#pending = { previous, route };
		queueMicrotask(() => {
			const pending = this.#pending;
			this.#pending = null;
			if (pending) void this.#enter(pending.previous, pending.route);
		});
	}

	async #enter(previous: Route | null, route: Route) {
		const generation = ++this.#generation;
		const stillCurrent = () => this.#generation === generation;
		const fallback = this.#fallback;
		this.#fallback = null;

		this.status = 'loading';
		this.error = null;
		try {
			await applyRoute(previous, route, stillCurrent);
			if (!stillCurrent()) return;
			this.status = 'ready';
			session.set('lastRoute', $state.snapshot(route));
		} catch (e) {
			if (!stillCurrent()) return;
			if (fallback) return this.go(fallback, true);
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		}
	}
}

export const router = new Router();
