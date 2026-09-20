import { characterStore } from '$lib/stores/character.svelte.js';
import { personaStore } from '$lib/stores/persona.svelte.js';
import type { Snowflake } from '$lib/types/core.js';

/**
 * Resolve the display-time `{{…}}` tokens in a message.
 *
 * `characterId` is null when the conversation has no character of its own — a
 * realm playthrough, where each turn belongs to a cast member instead. The
 * persona still resolves through the global active one; the character half
 * simply has nothing to point at.
 */
export function createPresentable(characterId: Snowflake | null, input: string): string {
	let output = input;

	// Replace `{{user:name}}` with the persona's configured name.
	// Prefers `display_name` and falls back to `name`. Also
	// respects overrides and falls back to the global active persona.
	const persona = characterId ? personaStore.overrideFor(characterId) : null;
	const activePersonaId = persona ? persona.persona_id : personaStore.activePersonaId;
	const activePersona = personaStore.loadedPersonas?.find((x) => x.id == activePersonaId);
	if (activePersona) {
		const relevantName = activePersona.display_name
			? activePersona.display_name
			: activePersona.name;
		output = output.replace('{{user:name}}', relevantName);
	}

	// Replace `{{char:name}}` with the persona's configured name.
	// Prefers `display_name` and falls back to `name`
	const character = characterId
		? characterStore.metadataList.find((x) => x.id === characterId)
		: undefined;
	if (character) {
		const relevantName = character.display_name ? character.display_name : character.name;
		output = output.replace('{{char:name}}', relevantName);
	}

	return output;
}
