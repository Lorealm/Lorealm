import type { ConversationMessage } from '#lib/types/conversation.js';

class ContextInspectorStore {
	message = $state<ConversationMessage | null>(null);

	get isOpen(): boolean {
		return this.message !== null;
	}

	open(message: ConversationMessage) {
		this.message = message;
	}

	close() {
		this.message = null;
	}
}

export const contextInspector = new ContextInspectorStore();
