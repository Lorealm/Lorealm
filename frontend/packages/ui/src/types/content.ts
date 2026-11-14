import { chatStore } from '#lib/stores/chat.svelte.js';
import { characterStore } from '#lib/stores/character.svelte.js';
import type { Snowflake } from '#lib/types/core.js';
import type { CharacterPart, TracedBlock } from '#lib/types/context.js';
import { t } from '#lib/stores/i18n.svelte.js';

export interface ResolvedContent {
	text: string;
	label: string;
	resolved: boolean;
}

function characterPartLabel(part: CharacterPart): string {
	switch (part.part) {
		case 'definition':
			return t.chat.inspector.refs.definition;
		case 'personality':
			return t.chat.inspector.refs.personality;
		case 'scenario':
			return t.chat.inspector.refs.scenario;
		case 'greeting':
			return t.chat.inspector.refs.greeting(part.order);
		case 'example':
			return t.chat.inspector.refs.example(part.order);
		default:
			return t.chat.inspector.refs.character;
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
			return {
				text: inline,
				label: t.chat.inspector.refs.unresolvedMessage,
				resolved: inline !== ''
			};
		}

		case 'character': {
			const text = resolveCharacterPart(ref.part);
			if (text !== null) return { text, label: characterPartLabel(ref.part), resolved: true };
			// Legacy fallback if present
			return { text: inline, label: characterPartLabel(ref.part), resolved: inline !== '' };
		}

		case 'persona': {
			const label = t.chat.inspector.refs.persona(ref.persona.name);
			const text = ref.persona.description ?? '';
			if (text !== '') return { text, label, resolved: true };
			// Legacy fallback if present
			return { text: inline, label, resolved: inline !== '' };
		}

		case 'plugin':
			return {
				text: inline,
				label: t.chat.inspector.refs.plugin(ref.source_id),
				resolved: inline !== ''
			};

		default:
			return { text: inline, label: '', resolved: inline !== '' };
	}
}
