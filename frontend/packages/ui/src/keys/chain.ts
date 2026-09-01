import type { Layer } from './types.js';

/** The layer every global command listens on. */
export const GLOBAL_LAYER = 'global';

function depthOf(element: HTMLElement): number {
	let depth = 0;
	for (let node: HTMLElement | null = element; node; node = node.parentElement) depth += 1;
	return depth;
}

/**
 * The layers a keystroke should be offered to, innermost first, ending with the
 * current view and then `global`.
 *
 * Which module is "focused" is read off the DOM rather than tracked in state,
 * so there is nothing to keep in sync and nothing to go stale.
 */
export function orderLayers(
	registered: readonly Layer[],
	target: Element | null,
	viewLayer: string
): Layer[] {
	const contained: Array<{ layer: Layer; depth: number }> = [];
	const front: Layer[] = [];
	const predicate: Layer[] = [];

	for (const layer of registered) {
		if (layer.element) {
			if (target && layer.element.contains(target)) {
				contained.push({ layer, depth: depthOf(layer.element) });
			} else if (layer.blocking) {
				front.push(layer);
			}
			continue;
		}

		if (layer.active && !layer.active()) continue;
		if (layer.blocking) front.push(layer);
		else predicate.push(layer);
	}

	contained.sort((a, b) => b.depth - a.depth);
	front.sort((a, b) => b.seq - a.seq);
	predicate.sort((a, b) => b.seq - a.seq);

	return [
		...front,
		...contained.map(({ layer }) => layer),
		...predicate,
		{ id: viewLayer, seq: -1 },
		{ id: GLOBAL_LAYER, seq: -2 }
	];
}
