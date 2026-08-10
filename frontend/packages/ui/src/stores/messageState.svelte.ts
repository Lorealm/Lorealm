import { useApi } from '$lib/api/client.js';
import type { Snowflake } from '$lib/types/core.js';
import { readPath, type ReconstructedState } from '$lib/types/state.js';
import { SvelteMap } from 'svelte/reactivity';

class MessageStateStore {
	#loadedStates = $state(new SvelteMap<Snowflake, ReconstructedState>());

	// The message currently pinned for inspection
	#targetId = $state<Snowflake | null>(null);
	#loading = $state(false);

	get #api() {
		return useApi();
	}

	async getState(
		messageId: Snowflake,
		conversationId?: Snowflake
	): Promise<ReconstructedState | undefined> {
		if (this.#loadedStates.has(messageId)) {
			return this.#loadedStates.get(messageId);
		}
		if (conversationId) {
			const state = await this.#api.chat.messages
				.state(conversationId, messageId)
				.catch(() => null);
			if (state) this.#loadedStates.set(messageId, state);
			return this.#loadedStates.get(messageId);
		}
		return undefined;
	}

	// Pin a message for the inspector and ensure its state is loaded
	async inspect(messageId: Snowflake, conversationId: Snowflake): Promise<void> {
		this.#targetId = messageId;
		if (this.#loadedStates.has(messageId)) return;
		this.#loading = true;
		try {
			await this.getState(messageId, conversationId);
		} finally {
			// Only clear if we're still the pinned message
			if (this.#targetId === messageId) this.#loading = false;
		}
	}

	clear(): void {
		this.#targetId = null;
	}

	get targetId(): Snowflake | null {
		return this.#targetId;
	}

	get loading(): boolean {
		return this.#loading;
	}

	get current(): ReconstructedState | undefined {
		return this.#targetId ? this.#loadedStates.get(this.#targetId) : undefined;
	}

	readVariable(path: string, messageId?: Snowflake): unknown {
		const sep = path.indexOf(':');
		if (sep === -1) return undefined;
		const namespace = path.slice(0, sep);
		const variableName = path.slice(sep + 1);
		if (!namespace || !variableName) return undefined;

		const state = messageId ? this.#loadedStates.get(messageId) : this.current;
		return readPath(state?.persistent[namespace], variableName);
	}

	// Formatted preview for a "namespace:variableName" path
	previewVariable(path: string, messageId?: Snowflake): string | undefined {
		const value = this.readVariable(path, messageId);
		return value === undefined ? undefined : previewValue(value);
	}
}

export const messageStateStore = new MessageStateStore();

function previewValue(value: unknown): string {
	let s: string;
	try {
		s = JSON.stringify(value);
	} catch {
		s = String(value);
	}
	if (s === undefined) s = String(value);
	return s.length > 120 ? s.slice(0, 117) + '…' : s;
}
