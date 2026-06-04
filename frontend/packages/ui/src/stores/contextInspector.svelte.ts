import type { ConversationMessage } from '$lib/types/conversation.js';

// One inspector is open at a time. The bubble button (deeply nested in an
// `#each`) calls `open`, and the modal — rooted up in ChatView — reads from
// here. Same singleton pattern as `chatStore` / `characterStore`, so no prop
// drilling through the message list.
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
