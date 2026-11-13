import { useApi } from '#lib/api/client.js';
import type { JsonValue } from '#lib/types/core.js';

type Listener = (name: string, payload: JsonValue) => void;

/** How long to wait before reopening a stream that ended. */
const RETRY_MS = 5000;

class PluginEvents {
	#listeners = new Set<{ handle: string; listener: Listener }>();
	#open = false;

	/** Calls `listener` with everything `handle`'s backend sends. */
	on(handle: string, listener: Listener): () => void {
		const entry = { handle, listener };
		this.#listeners.add(entry);
		if (!this.#open) {
			this.#open = true;
			void this.#run();
		}
		return () => this.#listeners.delete(entry);
	}

	async #run(): Promise<never> {
		while (true) {
			try {
				for await (const event of useApi().plugins.events()) {
					for (const { handle, listener } of this.#listeners) {
						if (handle !== event.handle) continue;
						try {
							listener(event.name, event.payload);
						} catch (e) {
							console.error(`plugin ${handle}: a listener for ${event.name} failed`, e);
						}
					}
				}
			} catch (e) {
				console.warn('The plugin event stream failed', e);
			}
			await new Promise((resolve) => setTimeout(resolve, RETRY_MS));
		}
	}
}

export const pluginEvents = new PluginEvents();
