import { PersistedRecord } from './persisted.svelte.js';

/**
 * The image behind the whole client.
 */
interface Backdrop {
	image: string | null;
}

function defaults(): Backdrop {
	return { image: null };
}

class BackdropStore {
	#record = new PersistedRecord<Backdrop>('backdrop', defaults());

	load(): Promise<void> {
		return this.#record.load();
	}

	get image(): string | null {
		return this.#record.current.image;
	}

	set(image: string): Promise<void> {
		return this.#record.set('image', image);
	}

	clear(): Promise<void> {
		return this.#record.set('image', null);
	}
}

export const backdrop = new BackdropStore();
