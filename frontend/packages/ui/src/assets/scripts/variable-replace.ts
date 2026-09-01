import { characterStore } from "$lib/stores/character.svelte.js";
import { personaStore } from "$lib/stores/persona.svelte.js";
import type { Snowflake } from "$lib/types/core.js";

export function createPresentable(characterId: Snowflake, input: string): string {
  // Ensure persona is loaded.
  // 
  // TODO: currently eagerly pulls the active persona. Should be cached in the future.
  personaStore.loadActive();
  
  let output = input;

  // Replace `{{user:name}}` with the persona's configured name.
  // Prefers `display_name` and falls back to `name`. Also
  // respects overrides and falls back to the global active persona.
  const persona = personaStore.overrideFor(characterId);
  const activePersonaId = persona ? persona.persona_id : personaStore.activePersonaId;
  const activePersona = personaStore.loadedPersonas?.find(x => x.id == activePersonaId);
  if (activePersona) {
    const relevantName = activePersona.display_name ? activePersona.display_name : activePersona.name;
    output = output.replace("{{user:name}}", relevantName);
  }

  // Replace `{{char:name}}` with the persona's configured name.
  // Prefers `display_name` and falls back to `name`
  const character = characterStore.metadataList.find(x => x.id === characterId);
  if (character) {
    const relevantName = character.display_name ? character.display_name : character.name;
    output = output.replace("{{char:name}}", relevantName);
  }
  
  return output;
}