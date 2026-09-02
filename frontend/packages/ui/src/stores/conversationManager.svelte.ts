import type { Snowflake } from '$lib/types/core.js';

class ConversationManagerStore {
	characterId = $state<Snowflake | null>(null);

	get isOpen(): boolean {
		return this.characterId !== null;
	}

	open(characterId: Snowflake) {
		this.characterId = characterId;
	}

	close() {
		this.characterId = null;
	}
}

export const conversationManager = new ConversationManagerStore();
