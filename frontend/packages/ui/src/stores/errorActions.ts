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

import { navigator } from '$lib/navigation/navigator.js';
import { SECRETS } from '$lib/navigation/routes.js';
import type { Snowflake } from '$lib/types/core.js';
import {
	extractError,
	type ErrorCode,
	type ErrorDetails,
	type ResourceKind
} from '$lib/types/error.js';
import { showToast, type ToastAction } from './errorToast.svelte.js';

/**
 * Where a resource kind lives, and what to call it in a button.
 *
 * `open` receives the id only when the resource still exists (see
 * {@link resourceAction}).
 */
interface Destination {
	noun: string;
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
	character: { noun: 'character', icon: SquareUser, open: studio('character') },
	character_version: { noun: 'character', icon: SquareUser, open: studio('character') },
	greeting: { noun: 'character', icon: SquareUser, open: studio('character') },
	lorebook: { noun: 'lorebook', icon: BookOpen, open: studio('lorebook') },
	lorebook_entry: { noun: 'lorebook', icon: BookOpen, open: studio('lorebook') },
	context_template: { noun: 'template', icon: ScrollText, open: studio('template') },
	template_node: { noun: 'template', icon: ScrollText, open: studio('template') },
	user_tool: { noun: 'tool', icon: Wrench, open: studio('tool') },
	persona: { noun: 'persona', icon: User, open: studio('persona') },
	user_widget: { noun: 'widget', icon: Puzzle, open: studio('widget') },
	realm: { noun: 'realm', icon: Users, open: (id) => navigator.start().realms(id) },
	realm_agent: { noun: 'realm', icon: Users, open: (id) => navigator.start().realms(id) },
	connection_profile: {
		noun: 'connection profile',
		icon: Plug,
		open: (id) => navigator.start().connections(id)
	},
	secret: {
		noun: 'API keys',
		icon: KeyRound,
		// The key manager is a single panel; individual keys aren't addressable.
		open: () => navigator.start().connections(SECRETS)
	},
	tokenizer: {
		noun: 'tokenizers',
		icon: Ruler,
		open: () => navigator.start().settings().user().tokenizers()
	}
};

/**
 * Builds a "take me there" action from whatever the server pointed at.
 *
 * A `404` means `details.id` names something that no longer exists, so the
 * action opens the surrounding list instead of deep-linking into a dead
 * resource.
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
		label: gone ? `Browse ${destination.noun}s` : `Open ${destination.noun}`,
		icon: destination.icon,
		tone: 'primary',
		run: () => destination.open(id)
	};
}

/**
 * Toast treatment for one server error code.
 *
 * This is the client half of the error contract: the server states what went
 * wrong, and this table decides how to present it. Only codes needing more than
 * {@link resourceAction} already provides belong here (see `docs/errors.md`).
 */
interface Recipe {
	title: string;
	/**
	 * Replaces the server message. Use only where the client knows something the
	 * server can't, Otherwise the server message is better, because it carries the
	 * specifics.
	 */
	message?: string;
	/** 0 keeps the toast up until dismissed. Use when the user must act. */
	durationMs?: number;
	/** Replaces the generic resource action. Omit to keep it. */
	actions?: ToastAction[];
}

const RECIPES: Partial<Record<ErrorCode, Recipe>> = {
	// Connection profiles: the first thing a new install has to set up.
	'state.no_active_connection_profile': {
		title: 'No connection selected',
		message:
			'A connection profile is needed to continue. Click the button below to open the Connections tab.',
		durationMs: 0
	},
	'connection.not_found': { title: 'Connection profile missing' },
	'connection.invalid_profile': { title: 'Connection profile incomplete', durationMs: 0 },
	'connection.secret_corrupted': { title: 'API key unreadable', durationMs: 0 },
	// Keeps the server message: it names what the provider actually rejected.
	'chat.provider_failed': { title: 'Model provider failed', durationMs: 0 },

	// Tokenizers: the second setup step, can be easy to miss
	'state.no_active_tokenizer': {
		title: 'No tokenizer selected',
		message: 'Choose a tokenizer to see token counts and stay inside the budget.',
		durationMs: 0
	},
	'tokenizer.not_found': { title: 'Tokenizer missing' },
	'tokenizer.fetch_failed': { title: 'Tokenizer download failed' },
	'tokenizer.invalid_source': { title: 'Tokenizer source invalid' },
	'tokenizer.invalid_data': { title: 'Tokenizer file invalid' }
};

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
	const recipe = err.code ? RECIPES[err.code] : undefined;
	const generic = resourceAction(err.details, err.httpStatus);

	// A recipe's actions replace the generic one; a caller's are added to it.
	const routed = recipe?.actions ?? (generic ? [generic] : []);

	return showToast({
		title: recipe?.title ?? fallback.title ?? 'Something went wrong',
		message: recipe?.message ?? err.message,
		variant: 'error',
		durationMs: recipe?.durationMs ?? fallback.durationMs ?? 6000,
		actions: [...routed, ...(fallback.actions ?? [])]
	});
}
