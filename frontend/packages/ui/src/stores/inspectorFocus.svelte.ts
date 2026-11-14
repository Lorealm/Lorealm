import { useApi } from '#lib/api/client.js';
import type { Character } from '#lib/types/character.js';
import type { Snowflake } from '#lib/types/core.js';

class InspectorFocusStore {
	characterId = $state<Snowflake | null>(null);
	character = $state<Character | null>(null);

	async toggleCharacter(id: Snowflake): Promise<void> {
		if (this.characterId === id) {
			this.clear();
			return;
		}
		this.characterId = id;
		this.character = null;
		try {
			const character = await useApi().characters.get(id);
			if (this.characterId === id) this.character = character;
		} catch {
			if (this.characterId === id) this.clear();
		}
	}

	clear() {
		this.characterId = null;
		this.character = null;
	}
}

export const inspectorFocus = new InspectorFocusStore();
