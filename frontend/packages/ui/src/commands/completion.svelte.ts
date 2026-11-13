import type { PickableResource } from '#lib/types/tools.js';
import { caretContext } from './caret.js';
import { resourceCatalog, resourceLabel } from './resources.svelte.js';
import { slashCommands } from './registry.svelte.js';
import { usage } from './types.js';

/** One row in the composer's popup */
export interface Suggestion {
	/** Stable key for the keyed each block. */
	key: string;
	/** A command's signature, or a resource's name. */
	primary: string;
	secondary: string;
	/** What replaces the span under the caret when this is accepted. */
	insert: string;
}

export type Completion =
	| { kind: 'none' }
	| { kind: 'command'; from: number; to: number; suggestions: Suggestion[] }
	| {
			kind: 'resource';
			from: number;
			to: number;
			resource: PickableResource;
			suggestions: Suggestion[];
	  };

/** What to offer for `text` with the caret at `caret`. */
export function completionAt(text: string, caret: number): Completion {
	const context = caretContext(text, caret);
	if (context.kind === 'none') return { kind: 'none' };

	if (context.kind === 'command') {
		return {
			kind: 'command',
			from: context.from,
			to: context.to,
			suggestions: slashCommands.matching(context.prefix).map((command) => ({
				key: command.name,
				primary: usage(command),
				secondary: command.description,
				insert: `/${command.name} `
			}))
		};
	}

	const command = slashCommands.resolve(context.command);
	const resource = command?.params?.find((p) => p.name === context.name)?.resource;
	if (!resource) return { kind: 'none' };

	return {
		kind: 'resource',
		from: context.from,
		to: context.to,
		resource,
		suggestions: resourceCatalog.search(resource, context.value).map((option) => ({
			key: String(option.id),
			primary: option.name,
			secondary: option.hint ?? resourceLabel(resource),
			insert: String(option.id)
		}))
	};
}
