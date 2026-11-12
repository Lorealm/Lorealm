import { session } from '$lib/stores/session.svelte.js';
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

interface HistoryMark {
	lorealmRoute: number;
}

const browserHistory = typeof window === 'undefined' ? null : window.history;

function markOf(state: unknown): number | null {
	const mark = (state as Partial<HistoryMark> | null)?.lorealmRoute;
	return typeof mark === 'number' ? mark : null;
}

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
	#studioSection: StudioSection = 'characters';
	#settingsScope: 'user' | 'server' = 'user';
	#userSection: UserSection = 'profile';
	#serverSection: ServerSection = 'hosting';

	status = $state<RouteStatus>('ready');
	error = $state<string | null>(null);

	/**
	 * The stack index the browser's own history sits at, so that its back and
	 * forward (a phone's back gesture, a mouse's side buttons) walk this stack
	 * instead of leaving the app. Null when the two have drifted apart.
	 */
	#browserAt: number | null = null;
	/** Traversals this router asked the browser for, whose `popstate` is an echo. */
	#echoes = 0;

	constructor() {
		if (typeof window === 'undefined') return;
		window.addEventListener('popstate', (event) => {
			if (this.#echoes > 0) {
				this.#echoes -= 1;
				return;
			}
			const mark = markOf(event.state);
			this.#browserAt = mark;
			// An entry from before a reload or a sign-out points past this stack.
			if (mark === null || mark === this.#index || mark >= this.#stack.length) return;
			const previous = this.current;
			this.#index = mark;
			this.#schedule(previous, this.current);
		});
	}

	#mark(replace: boolean) {
		if (!browserHistory) return;
		const mark: HistoryMark = { lorealmRoute: this.#index };
		if (replace) browserHistory.replaceState(mark, '');
		else browserHistory.pushState(mark, '');
		this.#browserAt = this.#index;
	}

	/** Move the browser along with a step taken here, if it was standing at `from`. */
	#follow(from: number, step: -1 | 1) {
		if (!browserHistory || this.#browserAt !== from) return;
		this.#browserAt = from + step;
		this.#echoes += 1;
		browserHistory.go(step);
	}

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
			this.#mark(false);
		}

		this.#remember(route);
		this.#schedule(previous, route);
	}

	back() {
		if (!this.canGoBack) return;
		const previous = this.current;
		this.#index -= 1;
		this.#follow(this.#index + 1, -1);
		this.#schedule(previous, this.current);
	}

	forward() {
		if (!this.canGoForward) return;
		const previous = this.current;
		this.#index += 1;
		this.#follow(this.#index - 1, 1);
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
		this.#mark(true);
		this.enter();
	}

	reset() {
		this.#stack = [HOME];
		this.#index = 0;
		this.#mark(true);
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
