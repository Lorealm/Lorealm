import { useApi } from '#lib/api/client.js';
import type { Snowflake } from '#lib/types/core.js';

/** A blocking plugin call in flight. */
interface Waiting {
	handle: string;
	conversationId: Snowflake | null;
}

/** The blocking plugin calls the client is waiting for. */
class PluginCalls {
	#waiting = $state<Waiting[]>([]);

	/** The plugin the user is waiting for in `conversationId`, if any. */
	waitingOn(conversationId: Snowflake | null): string | null {
		return this.#waiting.find((w) => w.conversationId === conversationId)?.handle ?? null;
	}

	/** Holds the chat until the returned function is called. */
	hold(handle: string, conversationId: Snowflake | null): () => void {
		const waiting: Waiting = { handle, conversationId };
		this.#waiting = [...this.#waiting, waiting];
		return () => {
			this.#waiting = this.#waiting.filter((w) => w !== waiting);
		};
	}

	/** Stops what the user is waiting for in `conversationId` */
	async cancel(conversationId: Snowflake | null): Promise<void> {
		if (conversationId === null || this.waitingOn(conversationId) === null) return;
		try {
			await useApi().chat.cancel(conversationId);
		} catch (e) {
			console.warn('Failed to stop the plugin call', e);
		}
	}
}

export const pluginCalls = new PluginCalls();
