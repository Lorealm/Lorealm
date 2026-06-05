import { chatStore } from '$lib/stores/chat.svelte.js';
import { characterStore } from '$lib/stores/character.svelte.js';
import type { Snowflake } from '$lib/types/core.js';
import type { CharacterPart, TracedBlock } from '$lib/types/context.js';

export interface ResolvedContent {
	text: string;
	label: string;
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
			// Legacy fallback if present
			return { text: inline, label: 'message · unresolved', resolved: inline !== '' };
		}

		case 'character': {
			const text = resolveCharacterPart(ref.part);
			if (text !== null) return { text, label: characterPartLabel(ref.part), resolved: true };
			// Legacy fallback if present
			return { text: inline, label: characterPartLabel(ref.part), resolved: inline !== '' };
		}

		case 'plugin':
			return { text: inline, label: `plugin · ${ref.source_id}`, resolved: inline !== '' };

		default:
			return { text: inline, label: '', resolved: inline !== '' };
	}
}
