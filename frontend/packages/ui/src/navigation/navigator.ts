import { router } from './router.svelte.js';
import type { Snowflake } from '$lib/types/core.js';
import { SECRETS } from './routes.js';
import type { Route, ServerSection, StudioSection, UserSection } from './routes.js';

class Chain {
	#started = false;

	go(route: Route) {
		router.go(route, this.#started);
		this.#started = true;
	}
}

/** The Studio sections that edit one resource, named by the kind they hold. */
export type StudioResource = 'character' | 'lorebook' | 'template' | 'tool' | 'persona' | 'widget';

const SECTION_FOR: Record<StudioResource, StudioSection> = {
	character: 'characters',
	lorebook: 'lorebooks',
	template: 'templates',
	tool: 'tools',
	persona: 'personas',
	widget: 'widgets'
};

class StudioStage {
	constructor(private readonly chain: Chain) {}

	#section(section: StudioSection, id?: Snowflake) {
		this.chain.go({ view: 'studio', section, id });
	}

	/** Open a resource whose kind is only known at runtime. */
	resource = (kind: StudioResource, id: Snowflake) => this.#section(SECTION_FOR[kind], id);

	realm = () => this.#section('realm');
	characters = (id?: Snowflake) => this.#section('characters', id);
	lorebooks = (id?: Snowflake) => this.#section('lorebooks', id);
	templates = (id?: Snowflake) => this.#section('templates', id);
	tools = (id?: Snowflake) => this.#section('tools', id);
	personas = (id?: Snowflake) => this.#section('personas', id);
	widgets = (id?: Snowflake) => this.#section('widgets', id);
}

class UserSettingsStage {
	constructor(private readonly chain: Chain) {}

	#section(section: UserSection, id?: string) {
		this.chain.go({ view: 'settings', scope: 'user', section, id });
	}

	profile = () => this.#section('profile');
	tokenizers = () => this.#section('tokenizers');
	plugins = (handle?: string) => this.#section('plugins', handle);
	appearance = () => this.#section('appearance');
	editor = () => this.#section('editor');
	inspector = () => this.#section('inspector');
	keybinds = () => this.#section('keybinds');
}

class ServerSettingsStage {
	constructor(private readonly chain: Chain) {}

	#section(section: ServerSection) {
		this.chain.go({ view: 'settings', scope: 'server', section });
	}

	hosting = () => this.#section('hosting');
	storage = () => this.#section('storage');
	sessions = () => this.#section('sessions');
	users = () => this.#section('users');
	accounts = () => this.#section('accounts');
	roles = () => this.#section('roles');
	characters = () => this.#section('characters');
	tokenizers = () => this.#section('tokenizers');
	tasks = () => this.#section('tasks');
}

class SettingsStage {
	constructor(private readonly chain: Chain) {}

	user(): UserSettingsStage {
		this.chain.go(router.recallUser());
		return new UserSettingsStage(this.chain);
	}

	server(): ServerSettingsStage {
		this.chain.go(router.recallServer());
		return new ServerSettingsStage(this.chain);
	}
}

/**
 * A destination's first stage.
 */
class Start {
	#chain = new Chain();

	write = () => this.#chain.go({ view: 'write' });
	characters = () => this.#chain.go({ view: 'characters' });
	realms = (id?: Snowflake) => this.#chain.go({ view: 'realms', id });
	connections = (id?: Snowflake | typeof SECRETS) => this.#chain.go({ view: 'connections', id });
	plugins = (handle?: string) => this.#chain.go({ view: 'plugins', id: handle });

	studio(): StudioStage {
		this.#chain.go(router.recallStudio());
		return new StudioStage(this.#chain);
	}

	settings(): SettingsStage {
		this.#chain.go(router.recallSettings());
		return new SettingsStage(this.#chain);
	}
}

/**
 * The way to move around the app.
 *
 * ```ts
 * navigator.start().studio().characters(id);
 * navigator.start().settings().server().sessions();
 * navigator.back();
 * ```
 *
 * A stage names one level of the destination and leaves the levels below it to
 * what you last visited. `.studio()` returns to the section you left, while
 * `.studio().characters()` names the section and clears the selection. Selection
 * is never remembered: use `back()` to return to something you had open.
 */
export const navigator = {
	start: () => new Start(),
	back: () => router.back(),
	forward: () => router.forward()
};
