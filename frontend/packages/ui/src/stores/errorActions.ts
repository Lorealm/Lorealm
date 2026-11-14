import {
	BookOpen,
	KeyRound,
	Plug,
	Puzzle,
	Ruler,
	ScrollText,
	SquareUser,
	User,
	Users,
	Wrench
} from '@lucide/svelte';
import type { Component } from 'svelte';

import { navigator } from '#lib/navigation/navigator.js';
import { SECRETS } from '#lib/navigation/routes.js';
import type { Snowflake } from '#lib/types/core.js';
import {
	extractError,
	type ErrorCode,
	type ErrorDetails,
	type ResourceKind
} from '#lib/types/error.js';
import { showToast, type ToastAction } from './errorToast.svelte.js';
import type { Messages } from '@lorealm/i18n';
import { t } from '#lib/stores/i18n.svelte.js';

/**
 * Where a resource kind lives, and what to call it in a button.
 *
 * `open` receives the id only when the resource still exists (see
 * {@link resourceAction}).
 */
type DestinationKey = keyof Messages['errors']['open'];

interface Destination {
	/** Picks the action's label. */
	key: DestinationKey;
	icon: Component;
	open: (id?: Snowflake) => void;
}

const studio = (kind: 'character' | 'lorebook' | 'template' | 'tool' | 'persona' | 'widget') => {
	const stage = () => navigator.start().studio();
	return (id?: Snowflake) => {
		const s = stage();
		switch (kind) {
			case 'character':
				return s.characters(id);
			case 'lorebook':
				return s.lorebooks(id);
			case 'template':
				return s.templates(id);
			case 'tool':
				return s.tools(id);
			case 'persona':
				return s.personas(id);
			case 'widget':
				return s.widgets(id);
		}
	};
};

const DESTINATIONS: Partial<Record<ResourceKind, Destination>> = {
	character: { key: 'character', icon: SquareUser, open: studio('character') },
	character_version: { key: 'character', icon: SquareUser, open: studio('character') },
	greeting: { key: 'character', icon: SquareUser, open: studio('character') },
	lorebook: { key: 'lorebook', icon: BookOpen, open: studio('lorebook') },
	lorebook_entry: { key: 'lorebook', icon: BookOpen, open: studio('lorebook') },
	context_template: { key: 'template', icon: ScrollText, open: studio('template') },
	template_node: { key: 'template', icon: ScrollText, open: studio('template') },
	user_tool: { key: 'tool', icon: Wrench, open: studio('tool') },
	persona: { key: 'persona', icon: User, open: studio('persona') },
	user_widget: { key: 'widget', icon: Puzzle, open: studio('widget') },
	realm: { key: 'realm', icon: Users, open: (id) => navigator.start().realms(id) },
	realm_agent: { key: 'realm', icon: Users, open: (id) => navigator.start().realms(id) },
	connection_profile: {
		key: 'connectionProfile',
		icon: Plug,
		open: (id) => navigator.start().connections(id)
	},
	secret: {
		key: 'secrets',
		icon: KeyRound,
		// The key manager is a single panel; individual keys aren't addressable.
		open: () => navigator.start().connections(SECRETS)
	},
	tokenizer: {
		key: 'tokenizers',
		icon: Ruler,
		open: () => navigator.start().settings().user().tokenizers()
	}
};

/**
 * Builds a "take me there" action from whatever the server pointed at.
 */
export function resourceAction(
	details: ErrorDetails | undefined,
	status: number | undefined
): ToastAction | undefined {
	const destination = details?.resource && DESTINATIONS[details.resource];
	if (!destination) return undefined;

	const gone = status === 404;
	const id = gone ? undefined : (details?.id as Snowflake | undefined);

	return {
		label: gone ? t.errors.browse[destination.key] : t.errors.open[destination.key],
		icon: destination.icon,
		tone: 'primary',
		run: () => destination.open(id)
	};
}

/**
 * Toast treatment for one server error code.
 */
interface Recipe {
	title: string;
	/**
	 * Replaces the server message. Use only where the client knows something the
	 * server can't.
	 */
	message?: string;
	/** 0 keeps the toast up until dismissed. Use when the user must act. */
	durationMs?: number;
	/** Replaces the generic resource action. Omit to keep it. */
	actions?: ToastAction[];
}

const recipes = (): Partial<Record<ErrorCode, Recipe>> => ({
	// Connection profiles: the first thing a new install has to set up.
	'state.no_active_connection_profile': {
		title: t.errors.noConnection.title,
		message: t.errors.noConnection.message,
		durationMs: 0
	},
	'connection.not_found': { title: t.errors.connectionMissing },
	'connection.invalid_profile': { title: t.errors.connectionIncomplete, durationMs: 0 },
	'connection.secret_corrupted': { title: t.errors.secretCorrupted, durationMs: 0 },
	// Keeps the server message: it names what the provider actually rejected.
	'chat.provider_failed': { title: t.errors.providerFailed, durationMs: 0 },

	// Tokenizers: the second setup step, can be easy to miss
	'state.no_active_tokenizer': {
		title: t.errors.noTokenizer.title,
		message: t.errors.noTokenizer.message,
		durationMs: 0
	},
	'tokenizer.not_found': { title: t.errors.tokenizerMissing },
	'tokenizer.fetch_failed': { title: t.errors.tokenizerFetchFailed },
	'tokenizer.invalid_source': { title: t.errors.tokenizerInvalidSource },
	'tokenizer.invalid_data': { title: t.errors.tokenizerInvalidData }
});

interface Fallback {
	title?: string;
	durationMs?: number;
	/** Shown alongside the resource action, not instead of it. */
	actions?: ToastAction[];
}

/**
 * Shows an error toast, routing the user to the fix when the server said what
 * the error was about.
 *
 * Prefer this over `showError` anywhere the thrown value came from the API.
 */
export function showApiError(e: unknown, fallback: Fallback = {}) {
	const err = extractError(e);
	const recipe = err.code ? recipes()[err.code] : undefined;
	const generic = resourceAction(err.details, err.httpStatus);

	// A recipe's actions replace the generic one; a caller's are added to it.
	const routed = recipe?.actions ?? (generic ? [generic] : []);

	return showToast({
		title: recipe?.title ?? fallback.title ?? t.errors.generic,
		message: recipe?.message ?? err.message,
		variant: 'error',
		durationMs: recipe?.durationMs ?? fallback.durationMs ?? 6000,
		actions: [...routed, ...(fallback.actions ?? [])]
	});
}
