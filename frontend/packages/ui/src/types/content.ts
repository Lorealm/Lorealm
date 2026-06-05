import { chatStore } from '$lib/stores/chat.svelte.js';
import { characterStore } from '$lib/stores/character.svelte.js';
import type { Snowflake } from '$lib/types/core.js';
import type { CharacterPart, TracedBlock } from '$lib/types/context.js';

// Turns a `TracedBlock` back into displayable text. Blocks store content by
// reference now, so the live text has to be looked back up from host state
// (the chat tree, the character). Read store state here so callers that invoke
// this inside a `$derived` stay reactive to edits/loads.

export interface ResolvedContent {
	/** Best display text. '' when a recoverable ref can't be reconstructed. */
	text: string;
	/** Short human label for where the content comes from. */
	label: string;
	/** True when `text` is real content (inline or a successful lookup), false
	 *  when we couldn't rebuild it and have nothing to show. */
	resolved: boolean;
}

function characterPartLabel(part: CharacterPart): string {
	switch (part.part) {
		case 'definition':
			return 'character · definition';
		case 'personality':
			return 'character · personality';
		case 'scenario':
			return 'character · scenario';
		case 'greeting':
			return `character · greeting #${part.order}`;
		case 'example':
			return `character · example #${part.order}`;
		default:
			return 'character';
	}
}

/** Rebuild a character part from the active character's definition. `order` on
 *  greetings/examples is a stable key, so we match it rather than index.
 *  Returns null when the character isn't loaded or the part can't be found. */
function resolveCharacterPart(part: CharacterPart): string | null {
	const def = characterStore.activeCharacter?.definition;
	if (!def) return null;
	switch (part.part) {
		case 'definition':
			return def.description;
		case 'personality':
			return def.personality;
		case 'scenario':
			return def.scenario;
		case 'greeting':
			return def.greetings.find((g) => g.order === part.order)?.text ?? null;
		case 'example':
			return def.example_messages.find((e) => e.order === part.order)?.content ?? null;
		default:
			return null;
	}
}

export function resolveBlockContent(block: TracedBlock): ResolvedContent {
	const ref = block.content_ref;
	const inline = block.content;

	switch (ref.kind) {
		case 'verbatim':
			return { text: inline, label: 'verbatim', resolved: true };

		case 'message': {
			const msg = chatStore.messageById(ref.id as unknown as Snowflake);
			if (msg) return { text: msg.content, label: 'message', resolved: true };
			// Legacy traces stored the text inline; fall back to it if present.
			return { text: inline, label: 'message · unresolved', resolved: inline !== '' };
		}

		case 'character': {
			const text = resolveCharacterPart(ref.part);
			if (text !== null) return { text, label: characterPartLabel(ref.part), resolved: true };
			// Character not loaded / part missing; fall back to inline if legacy.
			return { text: inline, label: characterPartLabel(ref.part), resolved: inline !== '' };
		}

		case 'plugin':
			return { text: inline, label: `plugin · ${ref.source_id}`, resolved: inline !== '' };

		default:
			return { text: inline, label: '', resolved: inline !== '' };
	}
}
