import { applyRoute } from './loader.svelte.js';
import {
	HOME,
	sameRoute,
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

	/** Sections to return to when a destination names a view but not a section. */
	#studioSection: StudioSection = 'lorebooks';
	#settingsScope: 'user' | 'server' = 'user';
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
		return this.#settingsScope === 'server' ? this.recallServer() : this.recallUser();
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
			if (route.scope === 'user') this.#userSection = route.section;
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

		this.status = 'loading';
		this.error = null;
		try {
			await applyRoute(previous, route, stillCurrent);
			if (!stillCurrent()) return;
			this.status = 'ready';
		} catch (e) {
			if (!stillCurrent()) return;
			this.error = e instanceof Error ? e.message : String(e);
			this.status = 'error';
		}
	}
}

export const router = new Router();
