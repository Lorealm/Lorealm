import { characterStore } from '#lib/stores/character.svelte.js';
import { personaStore } from '#lib/stores/persona.svelte.js';
import type { Snowflake } from '#lib/types/core.js';

export interface MacroContext {
	char: { name: string } | null;
	user: { name: string } | null;
}

/**
 * Build the [`MacroContext`] for a conversation.
 *
 * `characterId` is null when the conversation has no character of its own — a
 * realm playthrough, where each turn belongs to a cast member instead. The
 * persona still resolves through the global active one; the character half
 * simply has nothing to point at.
 */
export function macroContext(characterId: Snowflake | null): MacroContext {
	const override = characterId ? personaStore.overrideFor(characterId) : null;
	const personaId = override ? override.persona_id : personaStore.activePersonaId;
	const persona = personaStore.loadedPersonas?.find((x) => x.id == personaId);

	// The character prefers `display_name` and falls back to `name`.
	const character = characterId
		? characterStore.metadataList.find((x) => x.id === characterId)
		: undefined;

	return {
		user: persona ? { name: persona.display_name || persona.name } : null,
		char: character ? { name: character.display_name || character.name } : null
	};
}

/** Resolve the display-time `{{…}}` tokens in a message. */
export function createPresentable(characterId: Snowflake | null, input: string): string {
	const ctx = macroContext(characterId);
	let output = input;
	if (ctx.user) {
		const name = ctx.user.name;
		output = output.replaceAll('{{user:name}}', () => name);
	}
	if (ctx.char) {
		const name = ctx.char.name;
		output = output.replaceAll('{{char:name}}', () => name);
	}
	return output;
}
