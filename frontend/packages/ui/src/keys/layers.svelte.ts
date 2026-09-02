import type { Action } from 'svelte/action';
import { router } from '$lib/navigation/router.svelte.js';
import { orderLayers } from './chain.js';
import type { Layer, LayerOptions } from './types.js';

export { GLOBAL_LAYER } from './chain.js';
export type { Layer, LayerOptions };

/**
 * The layers currently able to claim a keystroke.
 *
 * Registration only; the ordering rule lives in {@link orderLayers}.
 */
class LayerStack {
	#layers = $state.raw<Layer[]>([]);
	#seq = 0;

	/** Add a layer. Call the returned function to remove it. */
	register(options: LayerOptions): () => void {
		const layer: Layer = { ...options, seq: this.#seq++ };
		this.#layers = [...this.#layers, layer];
		return () => {
			this.#layers = this.#layers.filter((candidate) => candidate !== layer);
		};
	}

	/** The most recently registered layer with this id, whether or not it is focused. */
	get(id: string): Layer | undefined {
		for (let i = this.#layers.length - 1; i >= 0; i--) {
			if (this.#layers[i].id === id) return this.#layers[i];
		}
		return undefined;
	}

	get all(): readonly Layer[] {
		return this.#layers;
	}

	/** The layer chain for a keystroke, innermost first. */
	chainFor(target: EventTarget | null): Layer[] {
		return orderLayers(
			this.#layers,
			target instanceof Element ? target : null,
			router.current.view
		);
	}
}

export const layers = new LayerStack();

/**
 * Marks an element as owning a keybind layer, active while focus is inside it.
 *
 * ```svelte
 * <div use:keyLayer={{ id: 'inspector' }}>
 * ```
 */
export const keyLayer: Action<HTMLElement, LayerOptions> = (node, options) => {
	let dispose = layers.register({ ...options, element: node });
	return {
		update(next: LayerOptions) {
			dispose();
			dispose = layers.register({ ...next, element: node });
		},
		destroy() {
			dispose();
		}
	};
};
